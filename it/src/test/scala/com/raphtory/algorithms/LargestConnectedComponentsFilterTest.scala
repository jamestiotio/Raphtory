package com.raphtory.algorithms

import com.raphtory.{BaseCorrectnessTest, TestQuery}
import com.raphtory.algorithms.filters.LargestConnectedComponentFilter
import com.raphtory.api.input.Source
import com.raphtory.sources.CSVEdgeListSource
import com.raphtory.spouts.ResourceOrFileSpout

class LargestConnectedComponentsFilterTest extends BaseCorrectnessTest {
  test("Test largest connected components filter") {
    correctnessTest(
            TestQuery(LargestConnectedComponentFilter(), 7),
            "ConnectedComponents/filterComponentsResults.csv"
    )
  }

  override def setSource(): Source = CSVEdgeListSource(ResourceOrFileSpout("/ConnectedComponents/twoComponents.csv"))
}
