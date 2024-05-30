use bevy_ecs::prelude::*;
use modula_core::{PreInit, ScheduleBuilder};
use modula_utils::HashMap;
use std::marker::PhantomData;

#[derive(Resource)]
pub struct Assets<T> {
    next: usize,
    assets: HashMap<usize, T>,
}

pub struct AssetId<T: Send + Sync + 'static>(usize, PhantomData<T>);

impl<T: Send + Sync + 'static> Clone for AssetId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + 'static> Copy for AssetId<T> {}

impl<T: Send + Sync + 'static> Assets<T> {
    pub fn new() -> Self {
        Self {
            next: 0,
            assets: HashMap::new(),
        }
    }

    /// Returns an empty [AssetId]
    pub fn add_empty(&mut self) -> AssetId<T> {
        self.next += 1;
        AssetId(self.next - 1, PhantomData)
    }

    /// Adds an asset and returns its id
    pub fn add(&mut self, asset: T) -> AssetId<T> {
        let id = self.add_empty();
        self.replace(id, asset);
        id
    }

    /// Immutably gets an asset from an id
    pub fn get(&self, asset_id: AssetId<T>) -> Option<&T> {
        self.assets.get(&asset_id.0)
    }

    /// Mutably gets an asset from an id
    pub fn get_mut(&mut self, asset_id: AssetId<T>) -> Option<&mut T> {
        self.assets.get_mut(&asset_id.0)
    }

    /// Puts a new value in an asset, all AssetIds pointing to the old asset will now point to the new asset
    pub fn replace(&mut self, asset_id: AssetId<T>, asset: T) -> Option<T> {
        self.assets.insert(asset_id.0, asset)
    }

    /// Removes an asset leaving None in its place, a new asset can be put in its place using replace
    pub fn remove(&mut self, asset_id: AssetId<T>) -> Option<T> {
        self.assets.remove(&asset_id.0)
    }
}

pub fn init_assets<T: Send + Sync + 'static>(schedule_builder: &mut ScheduleBuilder) {
    schedule_builder.add_systems(PreInit, |mut commands: Commands| {
        commands.insert_resource(Assets::<T>::new());
    })
}

/// Type that references a world and allows easy immutable access to all resources
pub struct AssetFetcher<'a> {
    world: &'a World,
}

impl<'a> AssetFetcher<'a> {
    /// Make a fetcher referencing a world
    pub fn new(world: &'a World) -> Self {
        AssetFetcher { world: &world }
    }

    /// Get an asset from the world being referenced
    pub fn get<T: Send + Sync + 'static>(&self, asset_id: AssetId<T>) -> Option<&'a T> {
        self.world.get_resource::<Assets<T>>()?.get(asset_id)
    }
}
