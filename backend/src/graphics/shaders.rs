use std::{collections::HashMap, fmt::Debug, marker::PhantomData};

use glam::Mat4;
use glow::{HasContext, WebProgramKey, WebShaderKey};
use web_sys::WebGlUniformLocation;

// use crate::world::ChunkPos;

use crate::core::position::ChunkPos;

use super::{Renderer, VertexAttrType};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum UniformTypes {
    // Texture,
    ModelMatrix,
    ViewMatrix,
    ProjMatrix,
    WorldPosition,
    PlayerChunkPos,
}

#[derive(Debug)]
pub struct ShaderFile {
    pub name: &'static str,
    pub source: &'static str,
}

pub struct ShaderDef<T> {
    pub(super) files: Vec<ShaderFile>,
    pub(super) attributes: Vec<(VertexAttrType, &'static str)>,
    pub(super) uniforms: Vec<(UniformTypes, &'static str)>,
    _phantom: PhantomData<T>,
}

impl<T> ShaderDef<T> {
    pub fn new(
        vertex_filename: Vec<ShaderFile>,
        attributes: Vec<(VertexAttrType, &'static str)>,
        uniforms: Vec<(UniformTypes, &'static str)>,
    ) -> Self {
        Self {
            files: vertex_filename,
            attributes,
            uniforms,
            _phantom: PhantomData,
        }
    }
}

// pub trait ShaderProgram {
//     type KeyType;

//     fn get_program<R>(&self) -> Self::KeyType;
// }
