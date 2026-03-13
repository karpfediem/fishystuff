use super::*;

pub(in crate::map::terrain::runtime) fn initialize_default_mode(
    mut mode: ResMut<ViewModeState>,
    config: Res<Terrain3dConfig>,
) {
    if config.enabled_default {
        mode.mode = ViewMode::Terrain3D;
    }
}

pub(in crate::map::terrain::runtime) fn spawn_terrain_light(mut commands: Commands) {
    commands.spawn((
        TerrainLightTag,
        world_3d_layers(),
        DirectionalLight {
            illuminance: 40_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.6, 0.0)),
        Visibility::Hidden,
    ));
}
