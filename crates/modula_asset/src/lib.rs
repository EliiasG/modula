use bevy_ecs::prelude::*;
use modula_core::{PreInit, ScheduleBuilder};
use modula_utils::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

#[derive(Resource)]
pub struct Assets<T> {
    next: usize,
    assets: HashMap<usize, T>,
}

pub struct AssetId<T: Send + Sync + 'static>(usize, PhantomData<T>);

impl<T: Send + Sync + 'static> Hash for AssetId<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: Send + Sync + 'static> PartialEq for AssetId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: Send + Sync + 'static> Eq for AssetId<T> {}

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

#[derive(SystemSet, Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub struct InitAssetsSet;

pub fn init_assets<T: Send + Sync + 'static>(schedule_builder: &mut ScheduleBuilder) {
    schedule_builder.add_systems(
        PreInit,
        (|mut commands: Commands| {
            commands.insert_resource(Assets::<T>::new());
        })
        .in_set(InitAssetsSet),
    );
}

pub trait AssetWorldExt {
    /// Adds an empty asset
    fn add_empty_asset<T: Send + Sync + 'static>(&mut self) -> AssetId<T>;
    /// Adds an asset and returns its id
    fn add_asset<T: Send + Sync + 'static>(&mut self, asset: T) -> AssetId<T>;
    /// Gets an asset from an id
    fn get_asset<T: Send + Sync + 'static>(&self, asset_id: AssetId<T>) -> Option<&T>;
    /// Gets an asset from an id and runs a function on it, if the asset is not found the function is not run
    fn with_asset<T: Send + Sync + 'static, F: FnOnce(&mut T)>(
        &mut self,
        asset_id: AssetId<T>,
        f: F,
    );
    /// Replaces an asset using [Assets::replace]
    fn replace_asset<T: Send + Sync + 'static>(
        &mut self,
        asset_id: AssetId<T>,
        asset: T,
    ) -> Option<T>;
    /// Removes an asset using [Assets::remove]
    fn remove_asset<T: Send + Sync + 'static>(&mut self, asset_id: AssetId<T>) -> Option<T>;
}

impl AssetWorldExt for World {
    fn add_empty_asset<T: Send + Sync + 'static>(&mut self) -> AssetId<T> {
        self.resource_mut::<Assets<T>>().add_empty()
    }

    fn add_asset<T: Send + Sync + 'static>(&mut self, asset: T) -> AssetId<T> {
        self.resource_mut::<Assets<T>>().add(asset)
    }

    fn get_asset<T: Send + Sync + 'static>(&self, asset_id: AssetId<T>) -> Option<&T> {
        self.get_resource::<Assets<T>>()?.get(asset_id)
    }

    fn with_asset<T: Send + Sync + 'static, F: FnOnce(&mut T)>(
        &mut self,
        asset_id: AssetId<T>,
        f: F,
    ) {
        self.get_resource_mut::<Assets<T>>()
            .map(|mut assets| assets.get_mut(asset_id).map(f));
    }

    fn replace_asset<T: Send + Sync + 'static>(
        &mut self,
        asset_id: AssetId<T>,
        asset: T,
    ) -> Option<T> {
        self.get_resource_mut::<Assets<T>>()?
            .replace(asset_id, asset)
    }

    fn remove_asset<T: Send + Sync + 'static>(&mut self, asset_id: AssetId<T>) -> Option<T> {
        self.get_resource_mut::<Assets<T>>()?.remove(asset_id)
    }
}
