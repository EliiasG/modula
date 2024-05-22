use bevy_ecs::prelude::*;
use modula_core::{PreInit, ScheduleBuilder};
use std::marker::PhantomData;

#[derive(Resource)]
pub struct Assets<T> {
    assets: Vec<Option<T>>,
}

#[derive(Clone, Copy)]
pub struct AssetId<T: Send + Sync + 'static>(usize, PhantomData<T>);

impl<T: Send + Sync + 'static> Assets<T> {
    pub fn new() -> Self {
        Self { assets: Vec::new() }
    }

    /// Adds an asset and returns its id, adding an asset reserves space in a vec, so calling this often will cause a memory leak
    pub fn add(&mut self, asset: T) -> AssetId<T> {
        self.assets.push(Some(asset));
        AssetId(self.assets.len() - 1, PhantomData)
    }

    /// Immutably gets an asset from an id
    pub fn get(&self, asset_id: AssetId<T>) -> Option<&T> {
        self.assets[asset_id.0].as_ref()
    }

    /// Mutably gets an asset from an id
    pub fn get_mut(&mut self, asset_id: AssetId<T>) -> Option<&mut T> {
        self.assets[asset_id.0].as_mut()
    }

    /// Puts a new value in an asset, all AssetIds pointing to the old asset will now point to the new asset
    pub fn replace(&mut self, asset_id: AssetId<T>, asset: T) -> Option<T> {
        self.assets[asset_id.0].replace(asset)
    }

    /// Removes an asset leaving None in its place, a new asset can be put in its place using replace
    pub fn remove(&mut self, asset_id: AssetId<T>) -> Option<T> {
        self.assets[asset_id.0].take()
    }
}

pub fn init_assets<T: Send + Sync + 'static>(schedule_builder: &mut ScheduleBuilder) {
    schedule_builder.add_system(PreInit, |mut commands: Commands| {
        commands.insert_resource(Assets::<T>::new());
    })
}
