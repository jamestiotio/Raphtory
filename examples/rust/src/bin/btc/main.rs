#![allow(unused_imports)]
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use raphtory::{core::utils::hashing, graph_loader::source::csv_loader::CsvLoader, prelude::*};
use regex::Regex;
use serde::Deserialize;
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{prelude::*, BufReader, LineWriter},
    marker::PhantomData,
    path::{Path, PathBuf},
    thread,
    thread::JoinHandle,
    time::Instant,
};

#[derive(Deserialize, std::fmt::Debug)]
pub struct Sent {
    addr: String,
    txn: String,
    amount_btc: u64,
    _amount_usd: f64,
    #[serde(with = "custom_date_format")]
    time: DateTime<Utc>,
}

#[derive(Deserialize, std::fmt::Debug)]
pub struct Received {
    _txn: String,
    _addr: String,
    _amount_btc: u64,
    _amount_usd: f64,
    #[serde(with = "custom_date_format")]
    _time: DateTime<Utc>,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let default_data_dir: PathBuf = [env!("CARGO_MANIFEST_DIR"), "src/bin/btc/data"]
        .iter()
        .collect();

    let data_dir = if args.len() < 2 {
        &default_data_dir
    } else {
        Path::new(args.get(1).unwrap())
    };

    if !data_dir.exists() {
        panic!("Missing data dir = {}", data_dir.to_str().unwrap())
    }

    let test_v = hashing::calculate_hash(&"139eeGkMGR6F9EuJQ3qYoXebfkBbNAsLtV:btc");

    // If data_dir/graphdb.bincode exists, use bincode to load the graph from binary encoded data files
    // otherwise load the graph from csv data files
    let encoded_data_dir = data_dir.join("graphdb.bincode");

    let graph = if encoded_data_dir.exists() {
        let now = Instant::now();
        let g = Graph::load_from_file(encoded_data_dir.as_path())
            .expect("Failed to load graph from encoded data files");

        println!(
            "Loaded graph from path {} with {} nodes, {} edges, took {} seconds",
            encoded_data_dir.to_str().unwrap(),
            g.count_nodes(),
            g.count_edges(),
            now.elapsed().as_secs()
        );

        g
    } else {
        let g = Graph::new();

        let now = Instant::now();

        CsvLoader::new(data_dir)
            .with_filter(Regex::new(r".+(sent|received)").unwrap())
            .load_into_graph(&g, |sent: Sent, g: &Graph| {
                let src = hashing::calculate_hash(&sent.addr);
                let dst = hashing::calculate_hash(&sent.txn);
                let time = sent.time.timestamp();

                if src == test_v || dst == test_v {
                    println!("{} sent {} to {}", sent.addr, sent.amount_btc, sent.txn);
                }

                g.add_edge(
                    time,
                    src,
                    dst,
                    [("amount".to_string(), Prop::U64(sent.amount_btc))],
                    None,
                )
                .unwrap();
            })
            .expect("Failed to load graph from CSV data files");

        println!(
            "Loaded graph from CSV data files {} with {} nodes, {} edges which took {} seconds",
            encoded_data_dir.to_str().unwrap(),
            g.count_nodes(),
            g.count_edges(),
            now.elapsed().as_secs()
        );

        g.save_to_file(encoded_data_dir)
            .expect("Failed to save graph");

        g
    };

    assert_eq!(graph.count_nodes(), 9132396);
    assert_eq!(graph.count_edges(), 5087223);

    let windowed_graph = graph.window(0, i64::MAX);

    assert!(windowed_graph.has_node(test_v));
    let v = windowed_graph.node(test_v).unwrap();

    let deg_out = v.out_edges().count();
    let deg_in = v.in_edges().count();

    assert_eq!(deg_out, 22);
    assert_eq!(deg_in, 1);
}

mod custom_date_format {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Utc.datetime_from_str(&s, FORMAT)
            .map_err(serde::de::Error::custom)
    }
}
