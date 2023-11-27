use std::{
    cell::RefCell,
    io,
    rc::Rc,
    str::{self, from_utf8, Utf8Error},
};

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt},
    ecs::system::SystemState,
    prelude::*,
    utils::BoxedFuture,
};
use bevy_egui::{egui, EguiContexts};
use bladeink::story::{
    errors::{ErrorHandler, ErrorType},
    Story,
};
use tap::Tap;
use thiserror::Error;

#[derive(Resource)]
pub(crate) struct StoryOutput(pub String);

#[derive(Default)]
pub(crate) struct JsonStoryLoader;

#[derive(Error, Debug)]
pub(crate) enum JsonStoryError {
    #[error("Could load json: {0}")]
    Io(#[from] io::Error),
    #[error("Utf8 error loading json: {0}")]
    Utf8(#[from] Utf8Error),
}

#[derive(Asset, TypePath, Debug)]
pub(crate) struct JsonStoryAsset(String);

impl AssetLoader for JsonStoryLoader {
    type Asset = JsonStoryAsset;
    type Error = JsonStoryError;
    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut bevy::asset::LoadContext,
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

#[derive(Resource)]
pub(crate) struct StoryJson(Handle<JsonStoryAsset>);

pub(crate) fn setup_story_asset(mut commands: Commands, server: Res<AssetServer>) {
    let handle = server.load("story.json");
    commands.insert_resource(StoryJson(handle));
}

pub(crate) fn setup_story(world: &mut World) {
    let mut system_state: SystemState<(Res<Assets<JsonStoryAsset>>, Res<StoryJson>)> =
        SystemState::new(world);
    let (assets, json) = system_state.get_mut(world);

    let story = assets.get(&json.0).unwrap();
    let story = Story::new(&story.0)
        .unwrap()
        .tap_mut(|s| s.set_error_handler(Rc::new(RefCell::new(Handler))));
    world.insert_non_send_resource(story);
    world.insert_resource(StoryOutput("".to_owned()));
}

pub(crate) fn show_story(
    mut contexts: EguiContexts,
    mut story: NonSendMut<Story>,
    mut story_output: ResMut<StoryOutput>,
) {
    egui::Window::new("Story").show(contexts.ctx_mut(), |ui| {
        if story.can_continue() {
            story_output.0 = story.continue_maximally().unwrap()
        }
        ui.label(&story_output.0);

        let choices = story.get_current_choices();
        for choice in choices {
            let text = &choice.text;
            if ui.button(text.clone()).clicked() {
                let index = *choice.index.borrow();
                story.choose_choice_index(index).unwrap();
            }
        }
    });
}

struct Handler;
impl ErrorHandler for Handler {
    fn error(&mut self, message: &str, error_type: ErrorType) {
        match error_type {
            ErrorType::Warning => eprintln!("Warning: {message}"),
            ErrorType::Error => panic!("Ink error: {message}"),
        }
    }
}
