use bevy::{
    asset::{
        io::{Reader, Writer},
        processor::LoadAndSave,
        saver::{AssetSaver, SavedAsset},
        Asset, AssetLoader, AsyncReadExt, AsyncWriteExt, LoadContext,
    },
    ecs::system::Resource,
    prelude::*,
    reflect::TypePath,
    utils::BoxedFuture,
};
use iyes_progress::prelude::*;
use std::{
    fs,
    fs::File,
    io::{self, Write},
    process::Command,
    str::{from_utf8, Utf8Error},
};
use tempfile::TempDir;
use thiserror::Error;

pub(crate) fn setup_story_asset(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut loading: ResMut<AssetsLoading>,
) {
    let handle = server.load("main.ink");
    commands.insert_resource(StoryJson(handle.clone()));
    loading.add(handle);
}

pub struct InkAssetPlugin;

impl Plugin for InkAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<JsonStoryLoader>()
            .init_asset::<JsonStoryAsset>()
            .init_asset_loader::<InkStoryLoader>()
            .init_asset::<InkStoryAsset>()
            .register_asset_processor::<LoadAndSave<InkStoryLoader, _>>(LoadAndSave::from(
                InkStorySaver,
            ))
            .set_default_asset_processor::<LoadAndSave<InkStoryLoader, InkStorySaver>>("ink");
    }
}

#[derive(Default)]
pub(crate) struct JsonStoryLoader;

#[derive(Default)]
pub(crate) struct InkStoryLoader;

#[derive(Default)]
pub(crate) struct InkStorySaver;

#[derive(Error, Debug)]
pub(crate) enum StoryAssetError {
    #[error("Could load story: {0}")]
    Io(#[from] io::Error),
    #[error("Utf8 error loading json: {0}")]
    Utf8(#[from] Utf8Error),
}

#[derive(Asset, TypePath, Debug)]
pub(crate) struct JsonStoryAsset(pub String);

#[derive(Asset, TypePath, Debug)]
pub(crate) struct InkStoryAsset(pub String);

#[derive(Resource)]
pub(crate) struct StoryJson(pub Handle<JsonStoryAsset>);

#[derive(Resource)]
pub(crate) struct StoryInk(pub Handle<InkStoryAsset>);

impl AssetLoader for JsonStoryLoader {
    type Asset = JsonStoryAsset;
    type Error = StoryAssetError;
    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let json = from_utf8(&bytes)?;

            Ok(JsonStoryAsset(json.to_owned()))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

impl AssetLoader for InkStoryLoader {
    type Asset = InkStoryAsset;
    type Error = StoryAssetError;
    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let json = from_utf8(&bytes)?;

            Ok(InkStoryAsset(json.to_owned()))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ink"]
    }
}

impl AssetSaver for InkStorySaver {
    type Asset = InkStoryAsset;
    type Error = StoryAssetError;
    type OutputLoader = JsonStoryLoader;
    type Settings = String;

    fn save<'a>(
        &'a self,
        writer: &'a mut Writer,
        asset: SavedAsset<'a, Self::Asset>,
        inklecate_path: &'a Self::Settings,
    ) -> BoxedFuture<'a, Result<<Self::OutputLoader as AssetLoader>::Settings, Self::Error>> {
        Box::pin(async move {
            let data = asset.0.as_bytes();

            let output_buffer;
            {
                let directory = TempDir::with_prefix("bevy-inklecate-")?;
                let path = directory.path();

                let out_path = path.join("output_ink");
                let in_path = path.join("input_ink");

                {
                    let mut in_file = File::create(&in_path)?;
                    in_file.write_all(data)?;
                    let out = Command::new(inklecate_path)
                        .arg("-j")
                        .arg("-o")
                        .arg(&out_path)
                        .arg(&in_path)
                        .output()?;
                    if !out.status.success() {
                        eprintln!(
                            "Inklecate: Status {} - Err {} - Out {}",
                            out.status,
                            from_utf8(&out.stderr)?,
                            from_utf8(&out.stdout)?
                        );
                    }
                    drop(in_file);
                }

                output_buffer = fs::read(out_path)?;
                directory.close()?;
            }

            writer.write_all(&output_buffer).await?;
            Ok(())
        })
    }
}
