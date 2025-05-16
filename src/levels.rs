use std::{marker::PhantomData, ops::Range};

use crate::{Icosphere, IcosphereVertex, triangle_count};

/// A collection of icosphere subdivisions, which can be used for rendering, similar to LODs.
/// We use terminology "levels", because LOD usually makes the mesh less detailed as it increases,
/// while this tesselates the mesh into more detailed shapes as the binning depth increases.  
#[derive(Debug, Clone)]
pub struct IcosphereLevels<T, S>
where
    T: IcosphereVertex,
    S: Icosphere<T>,
{
    /// The actual icospheres. Stores (max_binning_depth - min_binning_depth) icospheres, because in order to
    /// subdivide, we can't skip icospheres even if `binning_depth_step != 1`.
    levels: Vec<S>,

    /// The binning depth at the 0th level.
    ///
    /// Must be greater than zero. That is to say, this can't include the base icosahedron, because
    /// the arrangement of triangles and vertices for it is not guaranteed in the same way as it is
    /// for its subdivisions.
    pub min_binning_depth: usize,

    /// The increase in binning depth for every increase in level.
    ///
    /// Must be chosen such that [`Self::max_binning_depth`] is exactly the binning depth at the
    /// highest level.
    pub binning_depth_step: usize,

    _phantom: PhantomData<T>,
}

impl<T, S> IcosphereLevels<T, S>
where
    T: IcosphereVertex,
    S: Icosphere<T>,
{
    /// Get the icosahedron at the specified level
    pub fn get(&self, level: usize) -> &S {
        &self.levels[level * self.binning_depth_step]
    }

    /// The binning depth at a specific detail level.
    pub fn binning_depth_at_level(&self, level: usize) -> usize {
        self.min_binning_depth + level * self.binning_depth_step
    }

    /// The level corresponding to the binning depth, if any
    pub fn level_of_binning_depth(&self, binning_depth: usize) -> Option<usize> {
        // Check bounds
        if binning_depth < self.min_binning_depth
            || binning_depth >= self.binning_depth_at_level(self.levels.len())
            || (binning_depth - self.min_binning_depth) % self.binning_depth_step != 0
        {
            return None;
        }

        Some((binning_depth - self.min_binning_depth) / self.binning_depth_step)
    }

    /// The number of triangles in each "chunk". This is useful if you want to split draw calls into
    /// chunks, e.g., for culling purposes, or for dynamic detail level.
    ///
    /// Note that this is the triangle count, not the vertex count.
    ///
    /// To get the chunk size in vertices, use `chunk_size * 3`. The slice of vertex indices for a chunk at `chunk_index`
    /// would be `indices[(chunk_index * 3)..((chunk_index + chunk_size) * 3)]`, where `indices` is a flattened version
    /// of [`Icosphere::triangles`].
    pub fn chunk_size(&self) -> usize {
        1 << (2 * self.binning_depth_step)
    }

    /// The number of chunks in a certain level. Chunks are indexed by `0..self.chunk_size()`. See [`Self::chunk_size()`].
    ///
    /// If using dynamic detail level, when rendering a chunk index for level `n`, the sub-chunks inside this chunk
    /// (or in other words, the range of child chunk indices that this chunk index covers for level `n + 1`) are
    /// `(chunk_index * chunk_size)..(chunk_index * chunk_size + chunk_size)`.
    pub fn chunk_count(&self, level: usize) -> usize {
        assert!(level > 0, "chunk_count is only defined at level > 0");

        let binning_depth = self.binning_depth_at_level(level - 1);
        triangle_count(binning_depth)
    }

    /// For `chunk_index` at `level`, returns the range of subchunk indexes at `level + 1`.
    /// For example, if [`Self::binning_depth_step`] == 1, `chunk_index` maps to
    /// `(chunk_index * 4)..(chunk_index * 4 + 4)`.
    pub fn subchunk_indices(&self, chunk_index: usize) -> Range<usize> {
        let size = self.chunk_size();
        let start = chunk_index * size;
        let end = start + size;

        start..end
    }
}
