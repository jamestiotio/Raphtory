import Version._

name := "example-twitter"
version := raphtoryVersion
organization := "com.raphtory"
scalaVersion := raphtoryScalaVersion
libraryDependencies += "com.raphtory" %% "core" % raphtoryVersion
resolvers += Resolver.mavenLocal
Compile / resourceDirectory := baseDirectory.value / "resources"
