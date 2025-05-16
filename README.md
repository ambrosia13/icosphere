# icosphere

An icosphere is a geodesic sphere made by subdividing the triangles of an icosahedron and normalizing the vertex positions. 

This crate provides an easy way to generate icospheres in a format that is directly compatible with GPU rendering. The icospheres generated are optimal and do not duplicate vertices or triangles.

It also provides an abstraction over a level-of-detail version of the icosphere, by layering multiple icosphere subdivisions on top of each other, similar to a quadtree.

Two types of icospheres are implemented:
- static icospheres, where every vertex and triangle is generated at once. This is convenient because the triangles are all in a contiguous array. However, as the number of subdivisions goes up, this very quickly reaches a humongous memory footprint
- sparse icospheres, where vertices and triangles are generated on-the-fly as needed. This easily maps to an LOD system when rendering.