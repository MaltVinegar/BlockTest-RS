
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables))]

use bevy:: {
	prelude::*,
	diagnostic::{
		Diagnostics,
		FrameTimeDiagnosticsPlugin
	}, reflect::erased_serde::__private::serde::__private::de
};
use bevy_pixel_camera::*;
use std::collections::HashMap;

const BACKGROUND_LAYER: f32 = 0.0;
const BLOCK_LAYER: f32 = 1.0;
const MOB_LAYER: f32 = 2.0;

fn main() {
	App::new()
		.insert_resource(ClearColor(Color::rgb(0.3, 0.6, 0.6)))
		.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
		.add_plugin(FrameTimeDiagnosticsPlugin::default())
		.add_plugin(PixelCameraPlugin)
		.add_plugin(PixelBorderPlugin {
			color: Color::rgb(0.1, 0.1, 0.1),
		})
		.add_startup_system(setup)
		.add_system(fps_update_system)
		.add_system(update_blocks)
		.add_system(player_input)
		.add_system(do_physics)
	.run();
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
			position: Vec2::new(0.0, 12.0),
			velocity: Vec2::new(0.0, 0.0),
			size: Vec2::new(0.0, 3.0),
			touching_grass: false,
			jump_state: JumpState::None,
			walk_state: WalkState::None,
		}
	}
}

impl Mob {
	fn block_position(&self) -> (i64, i64) {
		(self.position.x.trunc() as i64, self.position.y.trunc() as i64)
	}
}

fn vec2_to_i64_loc(loc: Vec2) -> (i64, i64) {
	(loc.x.trunc() as i64, loc.y.trunc() as i64)
}

#[derive(Component, Default)]
struct Player;

#[derive(Component)]
struct PlayerAnimation {
	current_frame: usize,
	idle_frame: usize,
	first_walk_frame: usize,
	last_walk_frame: usize,
}

impl<'a> Default for PlayerAnimation {
	fn default() -> Self {
		Self {
			current_frame: 0,
			idle_frame: 0,
			first_walk_frame: 1,
			last_walk_frame: 7
		}
	}
}

#[derive(Component, Deref, DerefMut)]
struct PlayerAnimationTimer(Timer);

#[derive(Component, Default)]
struct Tile {
	id: usize,
	smooths: bool,
}

#[derive(Component)]
struct Chunk {
	tiles: [Entity; Chunk::SIZE],
	tile_map: HashMap<Entity, usize>,
	x_pos: i64,
	y_pos: i64,
}

#[derive(Component)]
struct FpsText;

impl Chunk {
	const WIDTH: usize = 64;
	const HEIGHT: usize = 64;
	const SIZE: usize = Self::WIDTH * Self::HEIGHT;

	fn new(tiles: [[Entity; Self::WIDTH]; Self::HEIGHT], x_pos: i64, y_pos: i64) -> Self {
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

fn tile_at(search_through: &Query<&Chunk>, absolute_x: i64, absolute_y: i64) -> Option<Entity> {
	let chunk_x: i64 =
		if absolute_x < 0 {
			((absolute_x + 1) / Chunk::WIDTH as i64) - 1
		} else {
			absolute_x / Chunk::WIDTH as i64
		};
	let local_x: usize =
		if (absolute_x % Chunk::WIDTH as i64) < 0 {
			((absolute_x % Chunk::WIDTH as i64) + 64) as usize
		} else {
			absolute_x as usize % Chunk::WIDTH
		};
	let chunk_y: i64 =
		if absolute_y < 0 {
			((absolute_y + 1)/ Chunk::WIDTH as i64) - 1
		} else {
			absolute_y / Chunk::WIDTH as i64
		};
	let local_y: usize =
		if (absolute_y % Chunk::HEIGHT as i64) < 0 {
			((absolute_y % Chunk::HEIGHT as i64) + 64) as usize
		} else {
			absolute_y as usize % Chunk::HEIGHT
		};
	for chunk in search_through {
		if chunk.x_pos == chunk_x && chunk.y_pos == chunk_y {
			if let Some(tile) = chunk.at(local_x, local_y) {
				return Some(tile);
			}
		}
	}
	None
}

fn find_tile(search_through: &Query<&Chunk>, to_find: Entity) -> Option<(i64, i64)> {
	for chunk in search_through {
		if let Some((x, y)) = chunk.find(to_find) {
			return Some((x as i64 + chunk.x_pos * 64, y as i64 + chunk.y_pos * 64));
		}
	}
	None
}

fn update_blocks(
	chunks: Query<&Chunk>,
	check: Query<&Tile>,
	player: Query<&Mob, With<Player>>,
	mut tiles: Query<(Entity, &Tile, &mut TextureAtlasSprite, &mut Transform)>,
) {
	for (entity, block, mut sprite, mut transfem) in &mut tiles {
		if block.smooths {
			if let Some((current_x, current_y)) = find_tile(&chunks, entity) {
				let mut dir_flags: usize = 0;
				if let Some(checking) = tile_at(&chunks, current_x, current_y + 1) {
					if let Ok(tile) = check.get(checking) {
						if tile.smooths {
							dir_flags |= 1;
						}
					}
				}
				if let Some(checking) = tile_at(&chunks, current_x, current_y - 1) {
					if let Ok(tile) = check.get(checking) {
						if tile.smooths {
							dir_flags |= 2;
						}
					}
				}
				if let Some(checking) = tile_at(&chunks, current_x + 1, current_y) {
					if let Ok(tile) = check.get(checking) {
						if tile.smooths {
							dir_flags |= 4;
						}
					}
				}
				if let Some(checking) = tile_at(&chunks, current_x - 1, current_y) {
					if let Ok(tile) = check.get(checking) {
						if tile.smooths {
							dir_flags |= 8;
						}
					}
				}
				if block.smooths {
					sprite.index = dir_flags;
				}
				if let Ok(player_loc) = player.get_single() {
					if let Some((relative_x, relative_y)) = find_tile(&chunks, entity) {
						transfem.translation = Vec3::new(
							(relative_x as f32 - player_loc.position.x) * 8.0,
							(relative_y as f32 - player_loc.position.y) * 8.0 - 8.0,
							BLOCK_LAYER
						);
					}
				}
			}
		}
	}
}

fn player_input (
	keys: Res<Input<KeyCode>>,
	time: Res<Time>,
	mut query: Query<&mut Mob, With<Player>>,
) {
	for mut mob in &mut query {
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
				_ => Vec2::new(mob.walk_state as i32 as f32 * 8.0, 0.0),
			}
			)
		} else {
			Vec2::new(0.0, -GRAVITY * time.delta_seconds() * 8.0)
		};
		let mut new_loc: Vec2 = mob.position + new_velocity * time.delta_seconds();
		mob.touching_grass = false;

		for y in 0..=mob.size.y.trunc() as i64 {
			for x in 0..=mob.size.x.trunc() as i64 {
				let checking_x = x + new_loc.x.trunc() as i64;
				let checking_y = y + new_loc.y.trunc() as i64;

				if let Some(tile_colliding) = tile_at(&chunks, checking_x, mob.position.y.trunc() as i64 + y) {
					if let Ok(tile) = blocks.get(tile_colliding) {
						if tile.smooths {
							new_velocity.x = 0.0;
							new_loc.x = mob.position.x.trunc();
						}
					}
				}
				if let Some(tile_colliding) = tile_at(&chunks, mob.position.x.trunc() as i64 + x, checking_y) {
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
			text.sections[1].value = format!("{:.2}:{:.2}", player.position.x, player.position.y)
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

	let air_spritesheet: Handle<TextureAtlas> = texture_atlases.add(air_atlas);
	let dirt_spritesheet: Handle<TextureAtlas> = texture_atlases.add(dirt_atlas);
	let _: Handle<TextureAtlas> = texture_atlases.add(glasspane_atlas);
	let grass_spritesheet: Handle<TextureAtlas> = texture_atlases.add(grass_atlas);
	let _: Handle<TextureAtlas> = texture_atlases.add(log_atlas);
	let _: Handle<TextureAtlas> = texture_atlases.add(stone_atlas);
	let _: Handle<TextureAtlas> = texture_atlases.add(stonebrick_atlas);
	let _: Handle<TextureAtlas> = texture_atlases.add(wood_atlas);

	commands.spawn((
		Player {},
		Mob {
			position: Vec2::new(0.0, 12.0),
			..default()
		},
		SpriteSheetBundle {
			texture_atlas: lizard_walk_atlas,
			transform: Transform {
				translation: Vec3 {
					y: 4.0,
					z: MOB_LAYER,
					..default()
				},
				..default()
			},
			..default()
		},
		PlayerAnimationTimer(Timer::from_seconds(0.05, TimerMode::Repeating)),
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
									Tile { id: 0, smooths: true },
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
									Tile { id: 0, smooths: true },
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
									Tile { id: 0, smooths: true },
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
									Tile { id: 0, smooths: true },
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
									Tile { id: 0, smooths: true },
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
									Tile { id: 0, smooths: true },
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

	commands.spawn(
		Chunk::new(
			chunk_data0,
			0, 0
		)
	);
	commands.spawn(
		Chunk::new(
			chunk_data1,
			-1, 0
		)
	);
	commands.spawn(
		Chunk::new(
			chunk_data2,
			1, 0
		)
	);

	commands.spawn(PixelCameraBundle::from_resolution(320, 240));
}
