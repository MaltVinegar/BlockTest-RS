
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables))]

use bevy:: {
	prelude::*,
	diagnostic::{
		Diagnostics,
		FrameTimeDiagnosticsPlugin
	},
};
use bevy_pixel_camera::*;
use std::collections::HashMap;

const BACKGROUND_LAYER: f32 = 0.0;
const BLOCK_LAYER: f32 = 1.0;
const MOB_LAYER: f32 = 2.0;

fn main() {
    static POSTSTARTUP: &str = "poststartup";
	App::new()
		.insert_resource(ClearColor(Color::rgb(0.3, 0.6, 0.6)))
		.add_startup_stage_after(StartupStage::PostStartup, POSTSTARTUP, SystemStage::parallel())

		.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
		.add_plugin(FrameTimeDiagnosticsPlugin::default())
		.add_plugin(PixelCameraPlugin)
		.add_plugin(PixelBorderPlugin {color: Color::rgb(0.1, 0.1, 0.1),})

		.add_startup_system(setup)
		.add_startup_system_to_stage(POSTSTARTUP, init_blocks)

		.add_system(fps_update_system)
		.add_system(update_blocks)
		.add_system(player_input)
		.add_system(do_physics)
		.add_system(walk_animation)
	.run();
}

#[derive(Resource)]
struct BlockId {
	block_by_ids: [(Handle<TextureAtlas>, Tile); BlockId::BLOCKS],
}

impl BlockId {
	const AIR: usize = 0;
	const DIRT: usize = 1;
	const GRASS: usize = 2;
	const BLOCKS: usize = 3;

	fn new(icons: [(Handle<TextureAtlas>, Tile); BlockId::BLOCKS]) -> Self {
		Self {
			block_by_ids: icons
		}
	}

	fn by_id(&self, id: usize) -> &(Handle<TextureAtlas>, Tile) {
		&self.block_by_ids[id]
	}
}

#[derive(Bundle, Default)]
struct PlayerBundle {
	mob: Mob,
	player: Player,
	player_animation: PlayerAnimation,
}

#[derive(Copy, Clone)]
enum JumpState {
	TryJump,
	None,
}

#[derive(Copy, Clone)]
enum WalkState {
	TryLeft = -1,
	None = 0,
	TryRight = 1,
}

#[derive(Component)]
struct Mob {
	position: Vec2,
	velocity: Vec2,
	size: Vec2,
	touching_grass: bool,
	jump_state: JumpState,
	walk_state: WalkState,
}

impl<'a> Default for Mob {
	fn default() -> Self {
		Self {
			position: Vec2::new(100.0, 140.0),
			velocity: Vec2::new(0.0, 0.0),
			size: Vec2::new(1.0, 3.0),
			touching_grass: false,
			jump_state: JumpState::None,
			walk_state: WalkState::None,
		}
	}
}

impl Mob {
	fn block_position(&self) -> (usize, usize) {
		(self.position.x.trunc() as usize, self.position.y.trunc() as usize)
	}
}

fn vec2_to_loc(loc: Vec2) -> (usize, usize) {
	(loc.x.trunc() as usize, loc.y.trunc() as usize)
}

#[derive(Component, Default)]
struct Player;

#[derive(Component)]
struct PlayerAnimation {
	idle_frame: usize,
	first_walk_frame: usize,
	last_walk_frame: usize,
}

impl<'a> Default for PlayerAnimation {
	fn default() -> Self {
		Self {
			idle_frame: 0,
			first_walk_frame: 1,
			last_walk_frame: 7
		}
	}
}

#[derive(Component, Deref, DerefMut)]
struct PlayerAnimationTimer(Timer);

#[derive(Component, Copy, Clone)]
struct Tile {
	id: usize,
	smooths: bool,
}

#[derive(Component)]
struct Chunk {
	tiles: [Entity; Chunk::SIZE],
	tile_map: HashMap<Entity, usize>,
	x_pos: usize,
	y_pos: usize,
}

#[derive(Component)]
struct FpsText;

impl Chunk {
	const WIDTH: usize = 64;
	const HEIGHT: usize = 64;
	const SIZE: usize = Self::WIDTH * Self::HEIGHT;

	fn new(tiles: [[Entity; Self::WIDTH]; Self::HEIGHT], x_pos: usize, y_pos: usize) -> Self {
		Self {
			tiles: core::array::from_fn(
				|i| tiles[i / Self::HEIGHT][i % Self::WIDTH]
			),
			tile_map: |tiles: &[[Entity; Self::WIDTH]; Self::HEIGHT]| -> HashMap<Entity, usize> {
				let mut rv: HashMap<Entity, usize> = HashMap::<Entity, usize>::new();
				for x in 0..tiles.len() {
					for y in 0..tiles[0].len() {
						rv.insert(tiles[y][x], x + y * Self::WIDTH);
					}
				}
				rv
			}(&tiles),
			x_pos,
			y_pos,
		}
	}

	fn at(&self, x: usize, y: usize) -> Option<Entity> {
		match x < Self::WIDTH && y < Self::HEIGHT {
			true => Some(self.tiles[x + y * Self::WIDTH]),
			false => None,
		}
	}

	fn at_mut(&mut self, x: usize, y: usize) -> Option<&mut Entity> {
		match x < Self::WIDTH && y < Self::HEIGHT {
			true => Some(&mut self.tiles[x + y * Self::WIDTH]),
			false => None,
		}
	}

	fn update_hashmap(&mut self) {
		self.tile_map = 
			|tiles: &[Entity; Self::SIZE]| -> HashMap<Entity, usize> {
				let mut rv: HashMap<Entity, usize> = HashMap::<Entity, usize>::new();
				for i in 0..tiles.len() {
					rv.insert(tiles[i], i);
				}
				rv
			}(&self.tiles)
	}

	fn find(&self, to_find: Entity) -> Option<(usize, usize)> {
		match self.tile_map.get(&to_find) {
			Some(found) => Some((found % Self::WIDTH, found / Self::HEIGHT)),
			None => None
		}
	}

	fn serialize(&self) {
		todo!();
	}

	fn deserialize() -> Self {
		todo!();
	}
}

fn tile_at(search_through: &Query<&Chunk>, absolute_x: usize, absolute_y: usize) -> Option<Entity> {
	let chunk_x: usize = absolute_x / Chunk::WIDTH;
	let local_x: usize = absolute_x % Chunk::WIDTH;
	let chunk_y: usize = absolute_y / Chunk::WIDTH;
	let local_y: usize = absolute_y % Chunk::HEIGHT;
	for chunk in search_through {
		if chunk.x_pos == chunk_x && chunk.y_pos == chunk_y {
			if let Some(tile) = chunk.at(local_x, local_y) {
				return Some(tile);
			}
		}
	}
	None
}

fn tile_at_mut(search_through: &Query<&mut Chunk>, absolute_x: usize, absolute_y: usize) -> Option<Entity> {
	let chunk_x: usize = absolute_x / Chunk::WIDTH;
	let local_x: usize = absolute_x % Chunk::WIDTH;
	let chunk_y: usize = absolute_y / Chunk::WIDTH;
	let local_y: usize = absolute_y % Chunk::HEIGHT;
	for chunk in search_through {
		if chunk.x_pos == chunk_x && chunk.y_pos == chunk_y {
			if let Some(tile) = chunk.at(local_x, local_y) {
				return Some(tile);
			}
		}
	}
	None
}

// Option<Entity>
// Despawn the return entity
fn set_tile_at(search_through: &mut Query<&mut Chunk>, tiles: &mut Query<(&Tile, &mut TextureAtlasSprite)>, smooths: bool,  replace_with: Entity, absolute_x: usize, absolute_y: usize) -> Option<(Entity, usize)> {
	let chunk_x: usize = absolute_x / Chunk::WIDTH;
	let local_x: usize = absolute_x % Chunk::WIDTH;
	let chunk_y: usize = absolute_y / Chunk::WIDTH;
	let local_y: usize = absolute_y % Chunk::HEIGHT;

	let check: [Option<Entity>; 4] = [
		tile_at_mut(search_through, absolute_x, absolute_y - 1),
		tile_at_mut(search_through, absolute_x, absolute_y + 1),
		tile_at_mut(search_through, absolute_x - 1, absolute_y),
		tile_at_mut(search_through, absolute_x + 1, absolute_y),
	];

	for mut chunk in search_through {
		if chunk.x_pos == chunk_x && chunk.y_pos == chunk_y {
			if let Some(tile) = chunk.at_mut(local_x, local_y) {
				let to_despawn: Entity = *tile;
				*tile = replace_with;
				let mut smoothed_sides: usize = 0;
				for checking in 0..check.len() {
					match check[checking] {
						Some(found) => 
							if let Ok((smoothing_tile, mut sprite)) = tiles.get_mut(found) {
								if smoothing_tile.smooths {
									if smooths {
										smoothed_sides |= 1 << checking;
										sprite.index |= 1 << checking;
									} else {
										sprite.index = sprite.index & ((1 | 2 | 4 | 8) ^ (1 << checking));
									}
								}
							},
						_ => (),
					}
				}
				let smoothed_sides_final: usize =
					((smoothed_sides & 1) << 1) |
					((smoothed_sides & 2) >> 1) |
					((smoothed_sides & 4) << 1) |
					((smoothed_sides & 8) >> 1);

				chunk.update_hashmap();
				return Some((to_despawn, smoothed_sides_final));
			}
		}
	}
	None
}

fn find_tile(search_through: &Query<&Chunk>, to_find: Entity) -> Option<(usize, usize)> {
	for chunk in search_through {
		if let Some((x, y)) = chunk.find(to_find) {
			return Some((x + chunk.x_pos * 64, y + chunk.y_pos * 64));
		}
	}
	None
}

fn update_blocks(
	chunks: Query<&Chunk>,
	check: Query<&Tile>,
	player: Query<&Mob, With<Player>>,
	mut tiles: Query<(Entity, &Tile, &mut Transform)>,
) {
	for (entity, block, mut transfem) in &mut tiles {
		if block.smooths {
			if let Some((current_x, current_y)) = find_tile(&chunks, entity) {
				if let Ok(player_loc) = player.get_single() {
					if let Some((relative_x, relative_y)) = find_tile(&chunks, entity) {
						transfem.translation = Vec3::new(
							(relative_x as f32 - player_loc.position.x) * 8.0,
							(relative_y as f32 - player_loc.position.y) * 8.0 - 12.0,
							BLOCK_LAYER
						);
					}
				}
			}
		}
	}
}

fn player_input (
	mut commands: Commands,
	keys: Res<Input<KeyCode>>,
	mouse_keys: Res<Input<MouseButton>>,
	time: Res<Time>,
	block_ids: Res<BlockId>,
	windows: Res<Windows>,
	pixel_projection: Query<&PixelProjection>,
	mut chunks: Query<&mut Chunk>,
	mut blocks: Query<(&Tile, &mut TextureAtlasSprite)>,
	mut query: Query<&mut Mob, With<Player>>,
) {
	if let Ok(mut mob) = query.get_single_mut() {
		if keys.just_pressed(KeyCode::Space) {
			mob.jump_state = JumpState::TryJump;
		}
		mob.walk_state = WalkState::None;
		if keys.pressed(KeyCode::A) {
			mob.walk_state = WalkState::TryLeft;
		}
		if keys.pressed(KeyCode::D) {
			mob.walk_state = WalkState::TryRight;
		}

		let window = windows.get_primary().unwrap();

		if mouse_keys.any_just_pressed([MouseButton::Left, MouseButton::Right]) {
			if let Some(cursor) = window.cursor_position() {
				if let Ok(projection) = pixel_projection.get_single() {
					let window_width: f32 = window.width();
					let window_height: f32 = window.height();
					let play_width: f32 = projection.desired_width.unwrap_or(0) as f32 * projection.zoom as f32 * 0.5;
					let play_height: f32 = projection.desired_height.unwrap_or(0) as f32 * projection.zoom as f32 * 0.5;
					let real_mouse_x: f32 = ((cursor.x - window_width * 0.5) / projection.zoom as f32).round();
					let real_mouse_y: f32 = ((cursor.y - window_height * 0.5) / projection.zoom as f32).round();

					if real_mouse_x.abs() <= play_width/2.0 && real_mouse_y.abs() <= play_height/2.0 {
						let clicked_block_x: usize = (mob.position.x + (real_mouse_x / 8.0)).round() as usize;
						let clicked_block_y: usize = (mob.position.y + (real_mouse_y + 12.0) / 8.0).round() as usize;

						let (handle, tile) = if mouse_keys.just_pressed(MouseButton::Left) {
							block_ids.by_id(1)
						} else {
							block_ids.by_id(0)
						};

						let new_block = commands.spawn((
							tile.clone(),
						)).id();

						if let Some((to_despawn, sides_smoothed)) = set_tile_at(&mut chunks, &mut blocks, tile.smooths, new_block, clicked_block_x, clicked_block_y) {
							commands.entity(to_despawn).despawn();

							commands.entity(new_block).insert( SpriteSheetBundle {
								texture_atlas: handle.clone(),
								transform: Transform {
									translation: Vec3::new(
										(clicked_block_x as f32 - mob.position.x) * 8.0,
										(clicked_block_y as f32 - mob.position.y) * 8.0 - 12.0,
										BLOCK_LAYER
									),
									..default()
								},
								sprite: TextureAtlasSprite {
									index: sides_smoothed,
									..default()
								},
								..default()
							});
						} else {
							commands.entity(new_block).despawn();
						}
					}
				}
			}
		}
	}
}

const GRAVITY: f32 = 9.81;

fn do_physics(
	time: Res<Time>,
	mut mob_query: Query<&mut Mob>,
	chunks: Query<&Chunk>,
	blocks: Query<&Tile>,
) {
	if let Ok(mut mob) = mob_query.get_single_mut() {
		let mut new_velocity = mob.velocity;
		new_velocity += if mob.touching_grass {
			(
			match mob.jump_state {
				JumpState::TryJump => {mob.jump_state = JumpState::None; Vec2::new(0.0, 30.0)},
				_ => Vec2::ZERO,
			} + match mob.walk_state {
				WalkState::None => Vec2::ZERO,
				_ => Vec2::new(mob.walk_state as i32 as f32 * 5.0, 0.0),
			}
			)
		} else {
			Vec2::ZERO
		} + Vec2::new(0.0, -GRAVITY * time.delta_seconds() * 8.0);
		let mut new_loc: Vec2 = mob.position + new_velocity * time.delta_seconds();
		mob.touching_grass = false;

		for y in 0..=mob.size.y.trunc() as usize {
			for x in 0..=mob.size.x.trunc() as usize {
				let checking_x = x + new_loc.x.trunc() as usize;
				let checking_y = y + new_loc.y.trunc() as usize;

				if let Some(tile_colliding) = tile_at(&chunks, checking_x, mob.position.y.trunc() as usize + y) {
					if let Ok(tile) = blocks.get(tile_colliding) {
						if tile.smooths {
							new_velocity.x = 0.0;
							new_loc.x = mob.position.x;
						}
					}
				}
				if let Some(tile_colliding) = tile_at(&chunks, mob.position.x.trunc() as usize + x, checking_y) {
					if let Ok(tile) = blocks.get(tile_colliding) {
						if tile.smooths {
							new_velocity.y = 0.0;
							new_loc.y = mob.position.y.trunc();
							new_velocity.x -= new_velocity.x * 10.0 * time.delta_seconds();
							mob.touching_grass = true;
						}
					}
				}
			}
		}

		mob.position = new_loc;
		mob.velocity = new_velocity;
	}
}

fn walk_animation(
	time: Res<Time>,
	mut query: Query<
		(
			&Mob,
			&mut Transform,
			&mut TextureAtlasSprite,
			&mut PlayerAnimationTimer,
			&PlayerAnimation
		),
		With<Player>
	>
) {
	for (mob, mut transfem, mut sprite, mut timer, data) in &mut query {
		match mob.walk_state {
			WalkState::None => (),
			_ => transfem.scale = Vec3::new(mob.walk_state as i32 as f32, 1.0, 1.0),
		};
		timer.tick(time.delta());
		if timer.just_finished() {
			if mob.touching_grass && !matches!(mob.walk_state, WalkState::None){
				if (sprite.index < data.first_walk_frame) || (sprite.index >= data.last_walk_frame) {
					sprite.index = data.first_walk_frame;
				} else {
					sprite.index += 1;
				}
			} else {
				sprite.index = data.idle_frame;
			}
		}
	}
}

fn fps_update_system(
	diagnostics: Res<Diagnostics>,
	player_query: Query<&Mob, With<Player>>,
	mut query: Query<
		&mut Text,
		With<FpsText>
	>
) {
	for mut text in &mut query {
		if let Ok(player) = player_query.get_single() {
			text.sections[1].value = format!("{:.2}:{:.2}:{}", player.position.x, player.position.y, player.touching_grass)
		}
//		if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
//			if let Some(value) = fps.smoothed() {
//				text.sections[1].value = format!("{value:.2}");
//			}
//		}
	}
}

fn setup (
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
	commands.spawn((
		TextBundle::from_sections([
			TextSection::new(
				"FPS: ",
				TextStyle {
					font: asset_server.load("UI/small_font.ttf"),
					font_size: 60.0,
					color: Color::WHITE,
				},
			),
			TextSection::from_style(TextStyle {
				font: asset_server.load("UI/small_font.ttf"),
				font_size: 60.0,
				color: Color::GOLD,
			}),
		]),
		FpsText,
	));

	let _: Handle<Image> = asset_server.load("title.png");
	let _: Handle<Image> = asset_server.load("titlebg.png");

	let _: Handle<Image> = asset_server.load("UI/button-down.png");
	let _: Handle<Image> = asset_server.load("UI/button-down2.png");
	let _: Handle<Image> = asset_server.load("UI/button-up.png");
	let _: Handle<Image> = asset_server.load("UI/selector.png");

	let _: Handle<Image> = asset_server.load("Sprites/delete.png");
	let _: Handle<Image> = asset_server.load("Sprites/player_human.png");
//	let _: Handle<Image> = asset_server.load("Sprites/player_lizard.png");
	let _: Handle<Image> = asset_server.load("Sprites/player_radlad.png");

	let player_walk_human: Handle<Image> = asset_server.load("Sprites/player_walk_human.png");
	let player_walk_lizard: Handle<Image> = asset_server.load("Sprites/player_lizard.png");
	let player_walk_radlad: Handle<Image> = asset_server.load("Sprites/player_walk_radlad.png");

	let player_walk_human_atlas: TextureAtlas = TextureAtlas::from_grid(//16x32
		player_walk_human,
		Vec2::new(16.0, 32.0),
		8,
		1,
		None,
		None
	);
	let player_walk_lizard_atlas: TextureAtlas = TextureAtlas::from_grid(//16x32
		player_walk_lizard,
		Vec2::new(16.0, 32.0),
		9,
		1,
		None,
		None
	);
	let player_walk_radlad_atlas: TextureAtlas = TextureAtlas::from_grid(//16x32
		player_walk_radlad,
		Vec2::new(16.0, 32.0),
		8,
		1,
		None,
		None
	);

	let _: Handle<TextureAtlas> = texture_atlases.add(player_walk_human_atlas);
	let lizard_walk_atlas: Handle<TextureAtlas> = texture_atlases.add(player_walk_lizard_atlas);
	let _: Handle<TextureAtlas> = texture_atlases.add(player_walk_radlad_atlas);

	let _: Handle<Image> = asset_server.load("Sprites/Blocks/glass.png");

/*	let pngs: [Handle<Image>; 7] = [
		asset_server.load("Sprites/Blocks/dirt.png"),
		asset_server.load("Sprites/Blocks/glasspane.png"),
		asset_server.load("Sprites/Blocks/grass.png"),
		asset_server.load("Sprites/Blocks/log.png"),
		asset_server.load("Sprites/Blocks/stone.png"),
		asset_server.load("Sprites/Blocks/stonebrick.png"),
		asset_server.load("Sprites/Blocks/wood.png"),
	];*/
	let air_png: Handle<Image> = asset_server.load("Sprites/Blocks/air.png");

	let dirt_png: Handle<Image> = asset_server.load("Sprites/Blocks/dirt.png");
	let glasspane_png: Handle<Image> = asset_server.load("Sprites/Blocks/glasspane.png");
	let grass_png: Handle<Image> = asset_server.load("Sprites/Blocks/grass.png");
	let log_png: Handle<Image> = asset_server.load("Sprites/Blocks/log.png");
	let stone_png: Handle<Image> = asset_server.load("Sprites/Blocks/stone.png");
	let stonebrick_png: Handle<Image> = asset_server.load("Sprites/Blocks/stonebrick.png");
	let wood_png: Handle<Image> = asset_server.load("Sprites/Blocks/wood.png");

	let air_atlas: TextureAtlas = TextureAtlas::from_grid(
		air_png,
		Vec2::splat(12.0),
		1,
		1,
		None,
		None,
	);
	let dirt_atlas: TextureAtlas = TextureAtlas::from_grid(//12x12
		dirt_png,
		Vec2::splat(12.0),
		4,
		4,
		None,
		None,
	);
	let glasspane_atlas: TextureAtlas = TextureAtlas::from_grid(//12x12
		glasspane_png,
		Vec2::splat(12.0),
		4,
		4,
		None,
		None,
	);
	let grass_atlas: TextureAtlas = TextureAtlas::from_grid(//12x12
		grass_png,
		Vec2::splat(12.0),
		4,
		4,
		None,
		None,
	);
	let log_atlas: TextureAtlas = TextureAtlas::from_grid(//12x12
		log_png,
		Vec2::splat(12.0),
		4,
		4,
		None,
		None,
	);
	let stone_atlas: TextureAtlas = TextureAtlas::from_grid(//12x12
		stone_png,
		Vec2::splat(12.0),
		4,
		4,
		None,
		None,
	);
	let stonebrick_atlas: TextureAtlas = TextureAtlas::from_grid(//12x12
		stonebrick_png,
		Vec2::splat(12.0),
		4,
		4,
		None,
		None,
	);
	let wood_atlas: TextureAtlas = TextureAtlas::from_grid(//12x12
		wood_png,
		Vec2::splat(12.0),
		4,
		4,
		None,
		None,
	);

	let air_spritesheet: Handle<TextureAtlas> = texture_atlases.add(air_atlas.clone());
	let dirt_spritesheet: Handle<TextureAtlas> = texture_atlases.add(dirt_atlas.clone());
	let _: Handle<TextureAtlas> = texture_atlases.add(glasspane_atlas);
	let grass_spritesheet: Handle<TextureAtlas> = texture_atlases.add(grass_atlas.clone());
	let _: Handle<TextureAtlas> = texture_atlases.add(log_atlas);
	let _: Handle<TextureAtlas> = texture_atlases.add(stone_atlas);
	let _: Handle<TextureAtlas> = texture_atlases.add(stonebrick_atlas);
	let _: Handle<TextureAtlas> = texture_atlases.add(wood_atlas);

	commands.insert_resource(
		BlockId::new(
			[
				(air_spritesheet.clone(), Tile {id: BlockId::AIR, smooths: false}),
				(dirt_spritesheet.clone(), Tile {id: BlockId::DIRT, smooths: true}),
				(grass_spritesheet.clone(), Tile {id: BlockId::GRASS, smooths: true}),
			]
		)
	);

	let camera = 
		commands.spawn(PixelCameraBundle::from_resolution(320, 240)).id();

	commands.spawn((
		Player {},
		Mob {
			..default()
		},
		SpriteSheetBundle {
			texture_atlas: lizard_walk_atlas.clone(),
			transform: Transform {
				translation: Vec3 {
					z: MOB_LAYER,
					..default()
				},
				..default()
			},
			..default()
		},
		PlayerAnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
		PlayerAnimation {
			..default()
		},
	));

	let chunk_data0: [[Entity; Chunk::WIDTH]; Chunk::HEIGHT] =
		core::array::from_fn(
			|y| -> [Entity; Chunk::WIDTH] {
				core::array::from_fn(
					|x| -> Entity {
						match y {
							0..=9 =>
								commands.spawn((
									SpriteSheetBundle {
										texture_atlas: dirt_spritesheet.clone(),
										transform: Transform {
											translation: Vec3 {
												z: BLOCK_LAYER,
												..default()
											},
											..default()
										},
										..default()
									},
									Tile { id: 1, smooths: true },
								)).id(),
							10 =>
								commands.spawn((
									SpriteSheetBundle {
										texture_atlas: grass_spritesheet.clone(),
										transform: Transform {
											translation: Vec3 {
												z: BLOCK_LAYER,
												..default()
											},
											..default()
										},
										..default()
									},
									Tile { id: 2, smooths: true },
								)).id(),
							11..=Chunk::HEIGHT =>
								commands.spawn((
									SpriteSheetBundle {
										texture_atlas: air_spritesheet.clone(),
										transform: Transform {
											translation: Vec3 {
												z: BLOCK_LAYER,
												..default()
											},
											..default()
										},
										..default()
									},
									Tile { id: 0, smooths: false },
								)).id(),
							_ => panic!(),
						}
					}
				)
			}
		);

	let chunk_data1: [[Entity; Chunk::WIDTH]; Chunk::HEIGHT] =
		core::array::from_fn(
			|y| -> [Entity; Chunk::WIDTH] {
				core::array::from_fn(
					|x| -> Entity {
						match y {
							0..=8 =>
								commands.spawn((
									SpriteSheetBundle {
										texture_atlas: dirt_spritesheet.clone(),
										transform: Transform {
											translation: Vec3 {
												z: BLOCK_LAYER,
												..default()
											},
											..default()
										},
										..default()
									},
									Tile { id: 1, smooths: true },
								)).id(),
							9 =>
								commands.spawn((
									SpriteSheetBundle {
										texture_atlas: grass_spritesheet.clone(),
										transform: Transform {
											translation: Vec3 {
												z: BLOCK_LAYER,
												..default()
											},
											..default()
										},
										..default()
									},
									Tile { id: 2, smooths: true },
								)).id(),
							10..=Chunk::HEIGHT =>
								commands.spawn((
									SpriteSheetBundle {
										texture_atlas: air_spritesheet.clone(),
										transform: Transform {
											translation: Vec3 {
												z: BLOCK_LAYER,
												..default()
											},
											..default()
										},
										..default()
									},
									Tile { id: 0, smooths: false },
								)).id(),
							_ => panic!(),
						}
					}
				)
			}
		);

	let chunk_data2: [[Entity; Chunk::WIDTH]; Chunk::HEIGHT] =
		core::array::from_fn(
			|y| -> [Entity; Chunk::WIDTH] {
				core::array::from_fn(
					|x| -> Entity {
						match y {
							0..=10 =>
								commands.spawn((
									SpriteSheetBundle {
										texture_atlas: dirt_spritesheet.clone(),
										transform: Transform {
											translation: Vec3 {
												z: BLOCK_LAYER,
												..default()
											},
											..default()
										},
										..default()
									},
									Tile { id: 1, smooths: true },
								)).id(),
							11 =>
								commands.spawn((
									SpriteSheetBundle {
										texture_atlas: grass_spritesheet.clone(),
										transform: Transform {
											translation: Vec3 {
												z: BLOCK_LAYER,
												..default()
											},
											..default()
										},
										..default()
									},
									Tile { id: 2, smooths: true },
								)).id(),
							12..=Chunk::HEIGHT =>
								commands.spawn((
									SpriteSheetBundle {
										texture_atlas: air_spritesheet.clone(),
										transform: Transform {
											translation: Vec3 {
												z: BLOCK_LAYER,
												..default()
											},
											..default()
										},
										..default()
									},
									Tile { id: 0, smooths: false },
								)).id(),
							_ => panic!(),
						}
					}
				)
			}
		);

	let chunk_data3: [[Entity; Chunk::WIDTH]; Chunk::HEIGHT] =
		core::array::from_fn( |y| -> [Entity; Chunk::WIDTH] {
			core::array::from_fn( |x| -> Entity {
				match x {
					0..=Chunk::HEIGHT =>
						commands.spawn((
							SpriteSheetBundle {
								texture_atlas: air_spritesheet.clone(),
								transform: Transform {
									translation: Vec3 {
										z: BLOCK_LAYER,
										..default()
									},
									..default()
								},
								..default()
							},
							Tile { id: 0, smooths: false },
						)).id(),
					_ => panic!(),
	}})});
	let chunk_data4: [[Entity; Chunk::WIDTH]; Chunk::HEIGHT] =
		core::array::from_fn( |y| -> [Entity; Chunk::WIDTH] {
			core::array::from_fn( |x| -> Entity {
				match x {
					0..=Chunk::HEIGHT =>
						commands.spawn((
							SpriteSheetBundle {
								texture_atlas: air_spritesheet.clone(),
								transform: Transform {
									translation: Vec3 {
										z: BLOCK_LAYER,
										..default()
									},
									..default()
								},
								..default()
							},
							Tile { id: 0, smooths: false },
						)).id(),
					_ => panic!(),
	}})});
	let chunk_data5: [[Entity; Chunk::WIDTH]; Chunk::HEIGHT] =
		core::array::from_fn( |y| -> [Entity; Chunk::WIDTH] {
			core::array::from_fn( |x| -> Entity {
				match x {
					0..=Chunk::HEIGHT =>
						commands.spawn((
							SpriteSheetBundle {
								texture_atlas: air_spritesheet.clone(),
								transform: Transform {
									translation: Vec3 {
										z: BLOCK_LAYER,
										..default()
									},
									..default()
								},
								..default()
							},
							Tile { id: 0, smooths: false },
						)).id(),
					_ => panic!(),
	}})});
	let chunk_data6: [[Entity; Chunk::WIDTH]; Chunk::HEIGHT] =
		core::array::from_fn( |y| -> [Entity; Chunk::WIDTH] {
			core::array::from_fn( |x| -> Entity {
				match x {
					0..=Chunk::HEIGHT =>
						commands.spawn((
							SpriteSheetBundle {
								texture_atlas: dirt_spritesheet.clone(),
								transform: Transform {
									translation: Vec3 {
										z: BLOCK_LAYER,
										..default()
									},
									..default()
								},
								..default()
							},
							Tile { id: 1, smooths: true },
						)).id(),
					_ => panic!(),
	}})});
	let chunk_data7: [[Entity; Chunk::WIDTH]; Chunk::HEIGHT] =
		core::array::from_fn( |y| -> [Entity; Chunk::WIDTH] {
			core::array::from_fn( |x| -> Entity {
				match x {
					0..=Chunk::HEIGHT =>
						commands.spawn((
							SpriteSheetBundle {
								texture_atlas: dirt_spritesheet.clone(),
								transform: Transform {
									translation: Vec3 {
										z: BLOCK_LAYER,
										..default()
									},
									..default()
								},
								..default()
							},
							Tile { id: 1, smooths: true },
						)).id(),
					_ => panic!(),
	}})});
	let chunk_data8: [[Entity; Chunk::WIDTH]; Chunk::HEIGHT] =
		core::array::from_fn( |y| -> [Entity; Chunk::WIDTH] {
			core::array::from_fn( |x| -> Entity {
				match x {
					0..=Chunk::HEIGHT =>
						commands.spawn((
							SpriteSheetBundle {
								texture_atlas: dirt_spritesheet.clone(),
								transform: Transform {
									translation: Vec3 {
										z: BLOCK_LAYER,
										..default()
									},
									..default()
								},
								..default()
							},
							Tile { id: 1, smooths: true },
						)).id(),
					_ => panic!(),
	}})});

	commands.spawn(
		Chunk::new(
			chunk_data6,
			1, 1
		)
	);
	commands.spawn(
		Chunk::new(
			chunk_data7,
			2, 1
		)
	);
	commands.spawn(
		Chunk::new(
			chunk_data8,
			3, 1
		)
	);
	commands.spawn(
		Chunk::new(
			chunk_data0,
			1, 2
		)
	);
	commands.spawn(
		Chunk::new(
			chunk_data1,
			2, 2
		)
	);
	commands.spawn(
		Chunk::new(
			chunk_data2,
			3, 2
		)
	);
	commands.spawn(
		Chunk::new(
			chunk_data3,
			1, 3
		)
	);
	commands.spawn(
		Chunk::new(
			chunk_data4,
			2, 3
		)
	);
	commands.spawn(
		Chunk::new(
			chunk_data5,
			3, 3
		)
	);

//	commands.spawn(PixelCameraBundle::from_resolution(320, 240));
}

fn init_blocks(
	chunks: Query<&Chunk>,
	check: Query<&Tile>,
	player: Query<&Mob, With<Player>>,
	mut tiles: Query<(Entity, &Tile, &mut TextureAtlasSprite)>,
) {
	for (entity, block, mut sprite) in &mut tiles {
		if block.smooths {
			if let Some((current_x, current_y)) = find_tile(&chunks, entity) {
				let mut smoothing_flags: usize = 0;
				if let Some(checking) = tile_at(&chunks, current_x, current_y + 1) {
					if let Ok(unwrapped) = check.get(checking) {
						if unwrapped.smooths {
							smoothing_flags |= 1;
						}
					}
				}
				if let Some(checking) = tile_at(&chunks, current_x, current_y - 1) {
					if let Ok(unwrapped) = check.get(checking) {
						if unwrapped.smooths {
							smoothing_flags |= 2;
						}
					}
				}
				if let Some(checking) = tile_at(&chunks, current_x + 1, current_y) {
					if let Ok(unwrapped) = check.get(checking) {
						if unwrapped.smooths {
							smoothing_flags |= 4;
						}
					}
				}
				if let Some(checking) = tile_at(&chunks, current_x - 1, current_y) {
					if let Ok(unwrapped) = check.get(checking) {
						if unwrapped.smooths {
							smoothing_flags |= 8;
						}
					}
				}
				sprite.index = smoothing_flags;
			}
		}
	}
}
