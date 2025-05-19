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

    /// The binning depth at the highest level.
    pub max_binning_depth: usize,

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
    /// Constructs all necessary icospheres given the minimum depth, the level count, and the depth step.
    /// The icospheres will be potentially empty/not generated yet.
    pub fn new(min_binning_depth: usize, level_count: usize, binning_depth_step: usize) -> Self {
        let max_binning_depth = min_binning_depth + (level_count - 1) * binning_depth_step;
        let mut levels = Vec::with_capacity(max_binning_depth - min_binning_depth + 1);

        for binning_depth in min_binning_depth..=max_binning_depth {
            levels.push(S::create(binning_depth));
        }

        Self {
            levels,
            min_binning_depth,
            max_binning_depth,
            binning_depth_step,
            _phantom: PhantomData,
        }
    }

    fn index_at_level(&self, level: usize) -> usize {
        level * self.binning_depth_step
    }

    /// Ensures the specified chunk is generated. Returns `true` if it was generated, and `false`
    /// if it has already been generated.
    pub fn update_chunk(&mut self, level: usize, chunk_index: usize) -> bool {
        let index = self.index_at_level(level);
        let (previous_levels, next_levels) = self.levels.split_at_mut(index);

        let previous = previous_levels.last().unwrap();
        let current = next_levels.first_mut().unwrap();

        current.subdivide_chunk(previous, chunk_index)
    }

    /// Get the icosahedron at the specified level
    pub fn get(&self, level: usize) -> &S {
        &self.levels[self.index_at_level(level)]
    }

    /// Same as [`Self::get`] but mutable
    pub fn get_mut(&mut self, level: usize) -> &mut S {
        let index = self.index_at_level(level);
        &mut self.levels[index]
    }

    /// Flattens the triangle indices into a contiguous array. Should be used for things like
    /// index buffers over this chunk.
    pub fn flattened_chunk_indices(&self, level: usize, chunk_index: usize) -> Vec<u32> {
        let mut indices = vec![0u32; 3 * self.chunk_size()]; // Multiplied by 3 for vertex count
        let subchunk_indices = self.subchunk_indices(chunk_index);

        for triangle_index in subchunk_indices {
            let ico = self.get(level);
            indices[(triangle_index * 3)..(triangle_index * 3 + 3)]
                .copy_from_slice(&ico.triangle(triangle_index));
        }

        indices
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

    /// The total number of levels.
    pub fn level_count(&self) -> usize {
        ((self.max_binning_depth - self.min_binning_depth) / self.binning_depth_step) + 1
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
    ///
    /// If level == 0, and it corresponds to a binning depth of zero, then the triangle count of the regular
    /// icosahedron is returned (20). That is to say, the regular icosahedron is not chunked at all, because
    /// there is no logical way to split its rendering into chunks (and it's a very simple mesh that should anyway
    /// be drawn all at once).
    pub fn chunk_count(&self, level: usize) -> usize {
        if self.binning_depth_at_level(level) == 0 {
            triangle_count(0)
        } else {
            let binning_depth = self.binning_depth_at_level(level - 1);
            triangle_count(binning_depth)
        }
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

    /// Returns a range of all chunk indices for the given level.
    pub fn chunk_indices(&self, level: usize) -> Range<usize> {
        0..self.chunk_count(level)
    }
}
