use bevy::text::Font;
use bevy::ui::UiTargetCamera;
use bevy_flair::prelude::*;

use crate::plugins::camera::UiCamera;
use crate::plugins::render_domain::{ui_layers, UiRenderEntity};
use crate::prelude::*;

mod setup;

#[derive(Component)]
pub(crate) struct UiRoot;

pub struct UiPlugin;

#[derive(Component, Debug, Default)]
pub struct UiPointerBlocker;

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct UiPointerCapture {
    pub blocked: bool,
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiFonts>()
            .init_resource::<UiPointerCapture>()
            .add_systems(Startup, setup::load_fonts.in_set(UiStartupSet))
            .add_systems(Startup, setup::setup_ui.after(UiStartupSet))
            .add_systems(
                Update,
                (
                    tag_new_ui_entities_with_ui_layer,
                    bind_ui_roots_to_ui_camera,
                ),
            );
    }
}

fn tag_new_ui_entities_with_ui_layer(mut commands: Commands, entities: NewUiEntityQuery<'_, '_>) {
    for entity in &entities {
        commands
            .entity(entity)
            .queue_silenced(|mut entity: EntityWorldMut| {
                entity.insert((UiRenderEntity, ui_layers()));
            });
    }
}

fn bind_ui_roots_to_ui_camera(
    mut commands: Commands,
    ui_camera_q: Query<Entity, With<UiCamera>>,
    ui_roots_q: Query<(Entity, Option<&UiTargetCamera>), With<UiRoot>>,
) {
    let Ok(ui_camera) = ui_camera_q.single() else {
        return;
    };
    for (entity, current_target) in &ui_roots_q {
        let already_bound = current_target
            .map(UiTargetCamera::entity)
            .map(|entity| entity == ui_camera)
            .unwrap_or(false);
        if !already_bound {
            commands
                .entity(entity)
                .queue_silenced(move |mut entity: EntityWorldMut| {
                    entity.insert(UiTargetCamera(ui_camera));
                });
        }
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct UiStartupSet;

type NewUiEntityQuery<'w, 's> = Query<
    'w,
    's,
    Entity,
    (
        Or<(Added<Node>, Added<Button>, Added<Text>, Added<ImageNode>)>,
        Without<UiRenderEntity>,
    ),
>;

#[derive(Resource, Clone, Default)]
pub struct UiFonts {
    pub regular: Handle<Font>,
}
