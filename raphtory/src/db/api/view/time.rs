use crate::{
    core::utils::time::{error::ParseTimeError, Interval, IntoTime},
    db::{
        api::view::internal::{OneHopFilter, TimeSemantics},
        graph::views::window_graph::WindowedGraph,
    },
};
use std::marker::PhantomData;

/// Trait defining time query operations
pub trait TimeOps<'graph> {
    type WindowedViewType: TimeOps<'graph> + 'graph;

    /// Return the timestamp of the default start for perspectives of the view (if any).
    fn start(&self) -> Option<i64>;

    /// Return the timestamp of the default for perspectives of the view (if any).
    fn end(&self) -> Option<i64>;

    /// Return the size of the window covered by this view
    fn window_size(&self) -> Option<u64> {
        match (self.start(), self.end()) {
            (Some(start), Some(end)) => Some((end - start) as u64),
            _ => None,
        }
    }

    /// Create a view including all events between `start` (inclusive) and `end` (exclusive)
    fn window<T: IntoTime>(&self, start: T, end: T) -> Self::WindowedViewType;

    /// Create a view that only includes events at `time`
    fn at<T: IntoTime>(&self, time: T) -> Self::WindowedViewType {
        let start = time.into_time();
        self.window(start, start.saturating_add(1))
    }

    /// Create a view that only includes events after `start` (exclusive)
    fn after<T: IntoTime>(&self, start: T) -> Self::WindowedViewType {
        let start = start.into_time().saturating_add(1);
        let end = self.end().unwrap_or(start.saturating_add(1));
        if end < start {
            self.window(start, start)
        } else {
            self.window(start, end)
        }
    }

    /// Create a view that only includes events before `end` (exclusive)
    fn before<T: IntoTime>(&self, end: T) -> Self::WindowedViewType {
        let end = end.into_time();
        let start = self.start().unwrap_or(end);
        if end < start {
            self.window(end, end)
        } else {
            self.window(start, end)
        }
    }

    /// Creates a `WindowSet` with the given `step` size    
    /// using an expanding window. The last window may fall partially outside the range of the data/view.
    ///
    /// An expanding window is a window that grows by `step` size at each iteration.
    fn expanding<I>(&self, step: I) -> Result<WindowSet<'graph, Self>, ParseTimeError>
    where
        Self: Sized + Clone + 'static,
        I: TryInto<Interval, Error = ParseTimeError>,
    {
        let parent = self.clone();
        match (self.start(), self.end()) {
            (Some(start), Some(end)) => {
                let step: Interval = step.try_into()?;

                Ok(WindowSet::new(parent, start, end, step, None))
            }
            _ => Ok(WindowSet::empty(parent)),
        }
    }

    /// Creates a `WindowSet` with the given `window` size and optional `step`
    /// using a rolling window. The last window may fall partially outside the range of the data/view.
    ///
    /// A rolling window is a window that moves forward by `step` size at each iteration.
    fn rolling<I>(
        &self,
        window: I,
        step: Option<I>,
    ) -> Result<WindowSet<'graph, Self>, ParseTimeError>
    where
        Self: Sized + Clone + 'static,
        I: TryInto<Interval, Error = ParseTimeError>,
    {
        let parent = self.clone();
        match (self.start(), self.end()) {
            (Some(start), Some(end)) => {
                let window: Interval = window.try_into()?;
                let step: Interval = match step {
                    Some(step) => step.try_into()?,
                    None => window,
                };
                Ok(WindowSet::new(parent, start, end, step, Some(window)))
            }
            _ => Ok(WindowSet::empty(parent)),
        }
    }
}

impl<'graph, V: OneHopFilter<'graph> + 'graph> TimeOps<'graph> for V {
    type WindowedViewType = V::Filtered<WindowedGraph<V::Graph>>;

    fn start(&self) -> Option<i64> {
        self.current_filter().view_start()
    }

    fn end(&self) -> Option<i64> {
        self.current_filter().view_end()
    }

    fn window<T: IntoTime>(&self, start: T, end: T) -> Self::WindowedViewType {
        self.one_hop_filtered(WindowedGraph::new(
            self.current_filter().clone(),
            start,
            end,
        ))
    }
}

#[derive(Clone)]
pub struct WindowSet<'graph, T> {
    view: T,
    cursor: i64,
    end: i64,
    step: Interval,
    window: Option<Interval>,
    _marker: PhantomData<&'graph T>,
}

impl<'graph, T: TimeOps<'graph> + Clone + 'graph> WindowSet<'graph, T> {
    fn new(view: T, start: i64, end: i64, step: Interval, window: Option<Interval>) -> Self {
        let cursor_start = start + step;
        Self {
            view,
            cursor: cursor_start,
            end,
            step,
            window,
            _marker: PhantomData,
        }
    }

    fn empty(view: T) -> Self {
        // timeline_start is greater than end, so no windows to return, even with end inclusive
        WindowSet::new(view, 1, 0, Default::default(), None)
    }

    // TODO: make this optionally public only for the development feature flag
    pub fn temporal(&self) -> bool {
        self.step.epoch_alignment
            || match self.window {
                Some(window) => window.epoch_alignment,
                None => false,
            }
    }

    /// Returns the time index of this window set
    pub fn time_index(&self, center: bool) -> TimeIndex<'graph, T> {
        TimeIndex {
            windowset: self.clone(),
            center,
        }
    }
}

pub struct TimeIndex<'graph, T> {
    windowset: WindowSet<'graph, T>,
    center: bool,
}

impl<'graph, T: TimeOps<'graph> + Clone + 'graph> Iterator for TimeIndex<'graph, T> {
    type Item = i64;

    fn next(&mut self) -> Option<Self::Item> {
        let center = self.center;
        self.windowset.next().map(move |view| {
            if center {
                view.start().unwrap() + ((view.end().unwrap() - view.start().unwrap()) / 2)
            } else {
                view.end().unwrap() - 1
            }
        })
    }
}

impl<'graph, T: TimeOps<'graph> + Clone + 'graph> Iterator for WindowSet<'graph, T> {
    type Item = T::WindowedViewType;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor < self.end + self.step {
            let window_end = self.cursor;
            let window_start = self
                .window
                .map(|w| window_end - w)
                .unwrap_or(self.view.start().unwrap_or(window_end));
            let window = self.view.window(window_start, window_end);
            self.cursor = self.cursor + self.step;
            Some(window)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod time_tests {
    use crate::{
        core::utils::time::TryIntoTime,
        db::{
            api::{
                mutation::AdditionOps,
                view::{time::WindowSet, TimeOps},
            },
            graph::graph::Graph,
        },
        prelude::{GraphViewOps, NO_PROPS},
    };
    use itertools::Itertools;

    // start inclusive, end exclusive
    fn graph_with_timeline(start: i64, end: i64) -> Graph {
        let g = Graph::new();
        g.add_node(start, 0, NO_PROPS).unwrap();
        g.add_node(end - 1, 0, NO_PROPS).unwrap();
        assert_eq!(g.start().unwrap(), start);
        assert_eq!(g.end().unwrap(), end);
        g
    }

    fn assert_bounds<'graph, G>(windows: WindowSet<'graph, G>, expected: Vec<(i64, i64)>)
    where
        G: GraphViewOps<'graph>,
    {
        let window_bounds = windows
            .map(|w| (w.start().unwrap(), w.end().unwrap()))
            .collect_vec();
        assert_eq!(window_bounds, expected)
    }

    #[test]
    fn rolling() {
        let g = graph_with_timeline(1, 7);
        let windows = g.rolling(2, None).unwrap();
        let expected = vec![(1, 3), (3, 5), (5, 7)];
        assert_bounds(windows, expected);

        let g = graph_with_timeline(1, 6);
        let windows = g.rolling(3, Some(2)).unwrap();
        let expected = vec![(0, 3), (2, 5), (4, 7)];
        assert_bounds(windows, expected.clone());

        let g = graph_with_timeline(0, 9).window(1, 6);
        let windows = g.rolling(3, Some(2)).unwrap();
        assert_bounds(windows, expected);
    }

    #[test]
    fn expanding() {
        let g = graph_with_timeline(1, 7);
        let windows = g.expanding(2).unwrap();
        let expected = vec![(1, 3), (1, 5), (1, 7)];
        assert_bounds(windows, expected);

        let g = graph_with_timeline(1, 6);
        let windows = g.expanding(2).unwrap();
        let expected = vec![(1, 3), (1, 5), (1, 7)];
        assert_bounds(windows, expected.clone());

        let g = graph_with_timeline(0, 9).window(1, 6);
        let windows = g.expanding(2).unwrap();
        assert_bounds(windows, expected);
    }

    #[test]
    fn rolling_dates() {
        let start = "2020-06-06 00:00:00".try_into_time().unwrap();
        let end = "2020-06-07 23:59:59.999".try_into_time().unwrap();
        let g = graph_with_timeline(start, end);
        let windows = g.rolling("1 day", None).unwrap();
        let expected = vec![
            (
                "2020-06-06 00:00:00".try_into_time().unwrap(), // entire 2020-06-06
                "2020-06-07 00:00:00".try_into_time().unwrap(),
            ),
            (
                "2020-06-07 00:00:00".try_into_time().unwrap(), // entire 2020-06-06
                "2020-06-08 00:00:00".try_into_time().unwrap(),
            ),
        ];
        assert_bounds(windows, expected);

        let start = "2020-06-06 00:00:00".try_into_time().unwrap();
        let end = "2020-06-08 00:00:00".try_into_time().unwrap();
        let g = graph_with_timeline(start, end);
        let windows = g.rolling("1 day", None).unwrap();
        let expected = vec![
            (
                "2020-06-06 00:00:00".try_into_time().unwrap(), // entire 2020-06-06
                "2020-06-07 00:00:00".try_into_time().unwrap(),
            ),
            (
                "2020-06-07 00:00:00".try_into_time().unwrap(), // entire 2020-06-07
                "2020-06-08 00:00:00".try_into_time().unwrap(),
            ),
        ];
        assert_bounds(windows, expected);

        // TODO: turn this back on if we bring bach epoch alignment for unwindowed graphs
        // let start = "2020-06-05 23:59:59.999".into_time().unwrap();
        // let end = "2020-06-07 00:00:00.000".into_time().unwrap();
        // let g = graph_with_timeline(start, end);
        // let windows = g.rolling("1 day", None).unwrap();
        // let expected = vec![
        //     (
        //         "2020-06-05 00:00:00".into_time().unwrap(), // entire 2020-06-06
        //         "2020-06-06 00:00:00".into_time().unwrap(),
        //     ),
        //     (
        //         "2020-06-06 00:00:00".into_time().unwrap(), // entire 2020-06-07
        //         "2020-06-07 00:00:00".into_time().unwrap(),
        //     ),
        // ];
        // assert_bounds(windows, expected);
    }

    #[test]
    fn expanding_dates() {
        let start = "2020-06-06 00:00:00".try_into_time().unwrap();
        let end = "2020-06-07 23:59:59.999".try_into_time().unwrap();
        let g = graph_with_timeline(start, end);
        let windows = g.expanding("1 day").unwrap();
        let expected = vec![
            (start, "2020-06-07 00:00:00".try_into_time().unwrap()),
            (start, "2020-06-08 00:00:00".try_into_time().unwrap()),
        ];
        assert_bounds(windows, expected);

        let start = "2020-06-06 00:00:00".try_into_time().unwrap();
        let end = "2020-06-08 00:00:00".try_into_time().unwrap();
        let g = graph_with_timeline(start, end);
        let windows = g.expanding("1 day").unwrap();
        let expected = vec![
            (start, "2020-06-07 00:00:00".try_into_time().unwrap()),
            (start, "2020-06-08 00:00:00".try_into_time().unwrap()),
        ];
        assert_bounds(windows, expected);
    }
}
