use std::{cell::RefCell, rc::Rc, str};

use crate::{
    input::Action,
    story::story_assets::{JsonStoryAsset, StoryJson},
    IntersectionsCount,
};
use bevy::{ecs::system::SystemState, prelude::*};
use bevy_egui::{egui, EguiContexts};
use bevy_inspector_egui::{inspector_options::ReflectInspectorOptions, InspectorOptions};
use bladeink::{
    story::{
        errors::{ErrorHandler, ErrorType},
        Story,
    },
    value_type::ValueType,
};
use itertools::Itertools;
use tap::Tap;

pub mod story_assets;

#[derive(Resource, Default, Reflect, Clone, Debug, InspectorOptions)]
#[reflect(Resource, InspectorOptions, Default)]
pub(crate) struct StoryOutput(pub String);

#[derive(Event)]
pub(crate) struct Tag(pub String);

pub(crate) fn story_needs_reload(
    mut asset_events: EventReader<AssetEvent<JsonStoryAsset>>,
) -> bool {
    for event in asset_events.read() {
        match event {
            AssetEvent::LoadedWithDependencies { .. } => {
                return true;
            }
            _ => {}
        }
    }
    return false;
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
    mut tag_events: EventWriter<Tag>,
    intersections: Res<IntersectionsCount>,
) {
    if story.can_continue() {
        while story.can_continue() {
            let output = &story.cont().unwrap();
            let tags = story.get_current_tags().unwrap();
            if tags.contains(&"CLEAR".to_string()) {
                story_output.0.clear()
            }
            story_output.0.push_str(output);
            dbg!(&story_output);
            dbg!(&tags);
            for tag in tags {
                tag_events.send(Tag(tag))
            }
        }
    }
    egui::Window::new("Story").show(contexts.ctx_mut(), |ui| {
        ui.label(&story_output.0);

        let choices = story.get_current_choices();
        for choice in choices {
            let text = &choice.text;
            let unsolved = intersections.0 > 0 && choice.tags.contains(&"SOLVED".to_string());
            let button = egui::Button::new(text.clone());
            if ui.add_enabled(!unsolved, button).clicked() {
                let index = *choice.index.borrow();
                story.choose_choice_index(index).unwrap();
                story_output.0.clear();
            }
        }
    });
}

pub(crate) fn log_tags(mut tag_events: EventReader<Tag>) {
    for tag in tag_events.read() {
        eprintln!("# {}", tag.0)
    }
}

pub(crate) fn tag_actions(mut tag_events: EventReader<Tag>, mut actions: EventWriter<Action>) {
    for Tag(tag) in tag_events.read() {
        match tag.as_str() {
            "RESET" => actions.send(Action::Reset),
            "ADD" => actions.send(Action::Bigger),
            string if string.starts_with("SIZE ") => {
                if let Some((graph_size, number_of_circles)) = string
                    .strip_prefix("SIZE ")
                    .unwrap()
                    .split(' ')
                    .flat_map(str::parse)
                    .tuples()
                    .next()
                {
                    actions.send(Action::Size(graph_size, number_of_circles))
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn update_intersections(
    mut story: NonSendMut<Story>,
    intersections_count: Res<IntersectionsCount>,
) {
    story
        .set_variable(
            "intersections",
            &ValueType::Int(intersections_count.0 as i32),
        )
        .unwrap()
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
