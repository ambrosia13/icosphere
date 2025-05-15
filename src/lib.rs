use std::collections::{HashMap, HashSet};

use glam::Vec3;

pub mod levels;

/// Vertex count of an icosphere at the given depth.
pub fn vertex_count(binning_depth: usize) -> usize {
    10 * (1 << (binning_depth * 2)) + 2
}

/// Triangle count of an icosphere at the given depth.
pub fn triangle_count(binning_depth: usize) -> usize {
    20 * (1 << (binning_depth * 2))
}

pub fn approximate_triangle_surface_area(binning_depth: usize, radius: f32) -> f32 {
    // Surface area of a sphere divided by the number of triangles
    (4.0 * std::f32::consts::PI * radius * radius) / triangle_count(binning_depth) as f32
}

/// The underlying storage for each icosphere vertex.
pub trait IcosphereVertex: Clone {
    fn position(&self) -> Vec3;
    fn from_position(position: Vec3, binning_depth: usize) -> Self;
}

impl IcosphereVertex for Vec3 {
    fn position(&self) -> Vec3 {
        *self
    }

    fn from_position(position: Vec3, _binning_depth: usize) -> Self {
        position
    }
}

pub trait Icosphere<T: IcosphereVertex> {
    /// The number of subdivisions from the regular icosahedron.
    fn binning_depth(&self) -> usize;

    /// Triangle at the given index.
    fn triangle(&self, triangle_index: usize) -> [u32; 3];

    /// List of vertices.
    fn vertices(&self) -> &[T];

    /// The total possible triangle count in an icosphere with the current binning depth.
    fn total_triangle_count(&self) -> usize;

    /// The total possible vertex count in an icosphere with the current binning depth.
    fn total_vertex_count(&self) -> usize;

    /// The amount of triangles actually in memory. If this icosphere is not sparse, this is equal to the total triangle count.
    fn allocated_triangle_count(&self) -> usize {
        self.total_triangle_count()
    }

    /// The amount of vertices actually in memory. If this is icosphere is not sparse, this is equal to the total vertex count.
    fn allocated_vertex_count(&self) -> usize {
        self.total_vertex_count()
    }

    /// Approximates the surface area of a triangle by dividing the sphere's surface area with the number of triangles.
    fn approximate_triangle_surface_area(&self, radius: f32) -> f32 {
        approximate_triangle_surface_area(self.binning_depth(), radius)
    }

    /// Subdivides `previous_triangles[parent_index]` into four children starting at `current_triangles[parent_index * 4]`.
    /// The previous binning depth must be 1 less than the current binning depth.
    fn subdivide_chunk(&mut self, previous: &Self, parent_index: usize);

    /// Subdivide the entire icosphere.
    fn subdivide(&self) -> Self;
}

/// A sphere-like shape constructed by subdividing a regular icosahedron and normalizing the resulting
/// coordinates.
///
/// All vertices and triangles are constructed for the entire shape upon creation. This can take huge
/// amounts of memory at high subdivisions. If you want to construct the triangles and vertices on-the-fly
/// to save memory, use a [`SparseIcosphere`].
#[derive(Debug, Clone)]
pub struct StaticIcosphere<T: IcosphereVertex> {
    /// The vertices may be in any order, don't rely on the order of this list.
    pub vertices: Vec<T>,

    /// Starting from the triangles of the regular icosahedron, each subdivision splits these triangles
    /// into groups of four. These four triangles are always contiguous and are indexed by the index
    /// of the parent icosahedron. For example, given the index `parent_triangle`, the four child
    /// triangles are located at `parent_triangle..parent_triangle + 4`.
    ///
    /// When rendering, if chunks are necessary, treat these groups of triangles as chunks.
    pub triangles: Vec<[u32; 3]>,

    /// For each vertex, this contains the five or six neighbor vertices.
    pub neighbors: HashMap<usize, HashSet<usize>>,

    /// Number of subdivisions from the regular icosahedron
    pub binning_depth: usize,

    /// Meaningless for the regular icosahedron.
    ///
    /// Used to avoid duplicating midpoints.
    midpoints: HashMap<(usize, usize), usize>,
}

impl<T: IcosphereVertex> StaticIcosphere<T> {
    /// The regular icosahedron of binning depth 0.
    pub fn regular() -> Self {
        let t = (1.0 + 5.0f32.sqrt()) / 2.0;

        let positions = [
            (Vec3::NEG_X + Vec3::Y * t).normalize(),
            (Vec3::X + Vec3::Y * t).normalize(),
            (Vec3::NEG_X + Vec3::NEG_Y * t).normalize(),
            (Vec3::X + Vec3::NEG_Y * t).normalize(),
            (Vec3::NEG_Y + Vec3::Z * t).normalize(),
            (Vec3::Y + Vec3::Z * t).normalize(),
            (Vec3::NEG_Y + Vec3::NEG_Z * t).normalize(),
            (Vec3::Y + Vec3::NEG_Z * t).normalize(),
            (Vec3::NEG_Z + Vec3::X * t).normalize(),
            (Vec3::Z + Vec3::X * t).normalize(),
            (Vec3::NEG_Z + Vec3::NEG_X * t).normalize(),
            (Vec3::Z + Vec3::NEG_X * t).normalize(),
        ];

        let triangles = vec![
            [0, 11, 5],
            [0, 5, 1],
            [0, 1, 7],
            [0, 7, 10],
            [0, 10, 11],
            [1, 5, 9],
            [5, 11, 4],
            [11, 10, 2],
            [10, 7, 6],
            [7, 1, 8],
            [3, 9, 4],
            [3, 4, 2],
            [3, 2, 6],
            [3, 6, 8],
            [3, 8, 9],
            [4, 9, 5],
            [2, 4, 11],
            [6, 2, 10],
            [8, 6, 7],
            [9, 8, 1],
        ];

        let neighbor_indices = [
            [11, 5, 1, 7, 10],
            [5, 9, 7, 8, 9],
            [11, 10, 3, 4, 6],
            [9, 4, 2, 6, 8],
            [5, 11, 3, 9, 2],
            [0, 11, 1, 9, 4],
            [10, 7, 3, 2, 8],
            [0, 1, 10, 6, 8],
            [3, 6, 7, 9, 1],
            [1, 5, 3, 8, 4],
            [0, 7, 11, 2, 6],
            [0, 5, 10, 4, 2],
        ];

        let vertices: Vec<T> = positions
            .into_iter()
            .map(|p| T::from_position(p, 0))
            .collect();

        let neighbors: HashMap<_, _> = neighbor_indices
            .into_iter()
            .map(|n| n.into_iter().collect::<HashSet<usize>>())
            .enumerate()
            .collect();

        Self {
            vertices,
            triangles,
            neighbors,
            binning_depth: 0,
            midpoints: HashMap::new(),
        }
    }

    pub fn nth(binning_depth: usize) -> Self {
        let mut ico = Self::regular();

        for _ in 0..binning_depth {
            ico = ico.subdivide();
        }

        ico
    }
}

impl<T: IcosphereVertex> Icosphere<T> for StaticIcosphere<T> {
    fn binning_depth(&self) -> usize {
        self.binning_depth
    }

    fn triangle(&self, triangle_index: usize) -> [u32; 3] {
        self.triangles[triangle_index]
    }

    fn vertices(&self) -> &[T] {
        &self.vertices
    }

    fn total_triangle_count(&self) -> usize {
        triangle_count(self.binning_depth)
    }

    fn total_vertex_count(&self) -> usize {
        vertex_count(self.binning_depth)
    }

    /// Doesn't do anything because static icospheres are already fully subdivided.
    fn subdivide_chunk(&mut self, _previous: &Self, _parent_index: usize) {}

    fn subdivide(&self) -> Self {
        let new_vertex_count = vertex_count(self.binning_depth + 1);
        let new_triangle_count = triangle_count(self.binning_depth + 1);

        let mut vertices: Vec<T> = Vec::with_capacity(new_vertex_count);
        let mut triangles: Vec<[u32; 3]> = Vec::with_capacity(new_triangle_count);
        let mut neighbors: HashMap<usize, HashSet<usize>> = HashMap::new();
        let mut midpoints = HashMap::new();

        vertices.extend_from_slice(&self.vertices);

        for parent_triangle in 0..self.triangles.len() {
            let [a, b, c] = self.triangles[parent_triangle];
            let [a, b, c] = [a as usize, b as usize, c as usize];

            let segments = [(a, b), (b, c), (c, a)];
            let mut segment_midpoints = [0u32; 3];

            for (edge_index, (i, j)) in segments.into_iter().enumerate() {
                let key = if i > j { (j, i) } else { (i, j) };

                let midpoint_index = match self.midpoints.get(&key) {
                    Some(&midpoint_index) => midpoint_index,
                    None => {
                        let midpoint =
                            (vertices[i].position() + vertices[j].position()).normalize();

                        let midpoint_index = vertices.len();
                        vertices.push(T::from_position(midpoint, self.binning_depth));

                        // Cache the midpoint so we don't duplicate when processing a different triangle
                        midpoints.insert(key, midpoint_index);

                        neighbors.entry(midpoint_index).or_default().insert(i);
                        neighbors.entry(midpoint_index).or_default().insert(j);
                        neighbors.entry(i).or_default().insert(midpoint_index);
                        neighbors.entry(j).or_default().insert(midpoint_index);

                        midpoint_index
                    }
                };

                segment_midpoints[edge_index] = midpoint_index as u32;

                let [a, b, c] = [a as u32, b as u32, c as u32];
                let [d, e, f] = segment_midpoints;

                triangles.push([a, d, f]);
                triangles.push([b, e, d]);
                triangles.push([c, f, e]);
                triangles.push([d, e, f]);
            }
        }

        Self {
            vertices,
            triangles,
            neighbors,
            binning_depth: self.binning_depth + 1,
            midpoints,
        }
    }
}

pub struct SparseIcosphere<T: IcosphereVertex> {
    /// Since vertices are added on-the-fly as needed, don't expect this to be in any particular order.
    pub vertices: Vec<T>,

    /// A sparse vector of triangle indices. Keys are in no particular order for the regular icosahedron,
    /// but are expanded fourfold for subdivisions. For example, for `triangle_index` in
    /// the regular icosahedron, its four subdivisions are `triangle_index..triangle_index + 4`.
    pub triangles: HashMap<usize, [u32; 3]>,

    /// Sparse vector of the neighbors of each vertex for this icosphere. The keys are vertex indices and
    /// the values are a collection of neighbor vertex indices.
    pub neighbors: HashMap<usize, HashSet<usize>>,

    /// Number of subdivisions from the regular icosahedron
    pub binning_depth: usize,

    /// Meaningless for the regular icosahedron. The keys are sorted pairs of vertex indices and the
    /// values are the vertex indices of the neighbor between them.
    midpoints: HashMap<(usize, usize), usize>,

    /// Meaningless for the regular icosahedron. The keys are vertex indices of the previous subdivision
    /// and the values are corresponding vertex indices of the current subdivision.
    previous_vertices: HashMap<u32, usize>,
}

impl<T: IcosphereVertex> SparseIcosphere<T> {
    /// Converts a [`StaticIcosphere`] into a [`SparseIcosphere`]. The resulting sparse icosphere
    /// is fully constructed (e.g. all triangles are filled)
    pub fn from_static(ico: StaticIcosphere<T>) -> Self {
        let vertices = ico.vertices;
        let triangles = ico.triangles.into_iter().enumerate().collect();
        let neighbors = ico.neighbors;
        let binning_depth = ico.binning_depth;
        let midpoints = ico.midpoints;

        Self {
            vertices,
            triangles,
            neighbors,
            binning_depth,
            midpoints,
            // The entire icosphere is constructed at once,
            // so there's no need for this cache
            previous_vertices: HashMap::new(),
        }
    }

    /// The regular icosahedron of binning depth 0.
    pub fn regular() -> Self {
        Self::from_static(StaticIcosphere::regular())
    }

    /// Construct a sparse icosphere at the given binning depth that is empty (i.e. not generated yet)
    pub fn empty(binning_depth: usize) -> Self {
        Self {
            vertices: Vec::new(),
            triangles: HashMap::new(),
            neighbors: HashMap::new(),
            binning_depth,
            midpoints: HashMap::new(),
            previous_vertices: HashMap::new(),
        }
    }

    /// Construct a sparse icosphere at the given binning depth that is filled (i.e. completely generated)
    pub fn filled(binning_depth: usize) -> Self {
        Self::from_static(StaticIcosphere::nth(binning_depth))
    }
}

impl<T: IcosphereVertex> Icosphere<T> for SparseIcosphere<T> {
    fn binning_depth(&self) -> usize {
        self.binning_depth
    }

    fn triangle(&self, triangle_index: usize) -> [u32; 3] {
        self.triangles[&triangle_index]
    }

    fn vertices(&self) -> &[T] {
        &self.vertices
    }

    fn total_triangle_count(&self) -> usize {
        triangle_count(self.binning_depth)
    }

    fn total_vertex_count(&self) -> usize {
        vertex_count(self.binning_depth)
    }

    fn allocated_triangle_count(&self) -> usize {
        self.triangles.len()
    }

    fn allocated_vertex_count(&self) -> usize {
        self.vertices.len()
    }

    fn subdivide_chunk(&mut self, previous: &Self, parent_index: usize) {
        // can only subdivide adjacent binning depths
        if previous.binning_depth + 1 != self.binning_depth {
            panic!("Attempted to subdivide icospheres with non-adjacent depth");
        }

        // all triangles generated
        if self.triangles.len() == self.total_triangle_count() {
            return;
        }

        // if teh base triangle is generated, assume the whole chunk is generated
        let all_triangles_present = self.triangles.contains_key(&(parent_index * 4));
        if all_triangles_present {
            return;
        }

        let mut new_vertex_indices = [0; 3];

        // Copy the previous polyhedron's triangle vertices to this polyhedron
        for (i, previous_vertex_index) in previous.triangles[&parent_index].into_iter().enumerate()
        {
            let vertex = previous.vertices[previous_vertex_index as usize].clone();

            new_vertex_indices[i] = match self.previous_vertices.get(&previous_vertex_index) {
                Some(&vertex_index) => vertex_index,
                None => {
                    let new_vertex_index = self.vertices.len();
                    self.vertices.push(vertex);

                    self.previous_vertices
                        .insert(previous_vertex_index, new_vertex_index);

                    new_vertex_index
                }
            };
        }

        // These vertex indices refer to self.vertices, not previous.vertices
        let [a, b, c] = new_vertex_indices;
        let segments = [(a, b), (b, c), (c, a)];

        let mut midpoints = [0; 3];

        for (edge_index, (i, j)) in segments.into_iter().enumerate() {
            let key = if i > j { (j, i) } else { (i, j) };

            let midpoint_index = match self.midpoints.get(&key) {
                Some(&midpoint_index) => midpoint_index,
                None => {
                    let midpoint = (previous.vertices[i].position()
                        + previous.vertices[j].position())
                    .normalize();

                    let midpoint_index = self.vertices.len();
                    self.vertices
                        .push(T::from_position(midpoint, self.binning_depth));

                    // Cache the midpoint so we don't duplicate when processing a different triangle
                    self.midpoints.insert(key, midpoint_index);

                    self.neighbors.entry(midpoint_index).or_default().insert(i);
                    self.neighbors.entry(midpoint_index).or_default().insert(j);
                    self.neighbors.entry(i).or_default().insert(midpoint_index);
                    self.neighbors.entry(j).or_default().insert(midpoint_index);

                    midpoint_index
                }
            };

            midpoints[edge_index] = midpoint_index as u32;
        }

        let [a, b, c] = [a as u32, b as u32, c as u32];
        let [d, e, f] = midpoints;

        // Each triangle gets four children, so we multiply the original index by four to have space
        let new_triangle_index = parent_index * 4;

        self.triangles.insert(new_triangle_index, [a, d, f]);
        self.triangles.insert(new_triangle_index + 1, [b, e, d]);
        self.triangles.insert(new_triangle_index + 2, [c, f, e]);
        self.triangles.insert(new_triangle_index + 3, [d, e, f]);
    }

    /// Requires this icosphere to be completely generated before subdividing
    fn subdivide(&self) -> Self {
        let mut ico = Self::empty(self.binning_depth + 1);

        for &chunk_index in self.triangles.keys() {
            ico.subdivide_chunk(self, chunk_index);
        }

        ico
    }
}
