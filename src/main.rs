use bevy:: {
	prelude::*,
	diagnostic::{
		Diagnostics,
		FrameTimeDiagnosticsPlugin
	}
};
use bevy_pixel_camera::*;
use std::collections::HashMap;

#[allow(dead_code)]
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
		.add_system(animate_character)
		.add_system(player_input)
		.add_system(do_momentum)
		.add_system(update_block)
		.add_system(move_player)
	.run();
}

#[derive(Component)]
struct Tile {
	smooths: bool,
}

//Todo: implement
#[derive(Component)]
struct TileBackground;

#[derive(Component)]
struct LoadedChunks {
	chunks: Vec<Entity>
}

impl LoadedChunks {
	fn new(initial_chunks: Vec<Entity>) -> LoadedChunks {
		LoadedChunks {
			chunks: initial_chunks
		}
	}

	fn at(&self, chunk_ids: &Query<&Chunk>, x: i64, y: i64) -> Option<Entity> {
		let chunk_x = x / 64;
		let local_x = x % 64;
		let chunk_y = y / 64;
		let local_y = y % 64;
		for &chunk in self.chunks.iter() {
			if let Ok(checking) = chunk_ids.get(chunk) {
				if chunk_x == checking.relative_x && chunk_y == checking.relative_y {
					return checking.at(local_x, local_y);
				}
			}
		}
		None
	}

	fn at_mut(&self, chunk_ids: &Query<&mut Chunk>, x: i64, y: i64) -> Option<Entity> {
		let chunk_x = x / 64;
		let local_x = x % 64;
		let chunk_y = y / 64;
		let local_y = y % 64;
		for &chunk in self.chunks.iter() {
			if let Ok(checking) = chunk_ids.get(chunk) {
				if (chunk_x == checking.relative_x) && (chunk_y == checking.relative_y) {
					return checking.at(local_x, local_y);
				}
			}
		}
		None
	}

	fn replace_at(&self, chunk_ids: &mut Query<&mut Chunk>, replace_with: Entity, x: i64, y: i64) -> Option<Entity> {
		let chunk_x = x / 64;
		let local_x = x % 64;
		let chunk_y = y / 64;
		let local_y = y % 64;
		for &chunk in self.chunks.iter() {
			if let Ok(mut checking) = chunk_ids.get_mut(chunk) {
				if (chunk_x == checking.relative_x) && (chunk_y == checking.relative_y) {
					return checking.replace_at(replace_with, local_x, local_y);
				}
			}
		}
		None
	}
}

#[derive(Component)]
struct Chunk {
	tiles: [Entity; Chunk::SIZE as usize],
	tile_map: HashMap<Entity, usize>,
//This is 64 tiles
	relative_x: i64,
	relative_y: i64,
}

impl Chunk {
	const WIDTH: i64 = 64;
	const HEIGHT: i64 = 64;
	const SIZE: usize = Chunk::WIDTH as usize * Chunk::HEIGHT as usize;

	fn new(data: [Entity; Chunk::SIZE as usize], x: i64, y: i64) -> Chunk {
		Chunk {
			tiles: data,
			tile_map: (
					|data: &[Entity; Chunk::SIZE]| {
						let mut rv :HashMap<Entity, usize> = HashMap::<Entity, usize>::new();
						for index in 0..data.len() {
							rv.insert(data[index], index);
						}
						rv
					}
				)(&data),
			relative_x: x,
			relative_y: y,
		}
	}

	fn at(&self, x: i64, y: i64) -> Option<Entity> {
		if x >= Chunk::WIDTH || y >= Chunk::HEIGHT || x < 0 || y < 0 {
			None
		} else {
			Some(self.tiles[(x + y * Chunk::WIDTH as i64) as usize])
		}
	}

	fn find(&self, finding: &Entity) -> Option<(usize, usize)> {
		match self.tile_map.get(&finding) {
			Some(index) => Some((index % Chunk::WIDTH as usize, index / Chunk::WIDTH as usize)),
			None => None,
		}
	}

	fn replace_at(&mut self, replace_with: Entity, x: i64, y: i64) -> Option<Entity> {
		if let Some(to_despawn) = self.at(x, y) {
			self.tiles[(x + y * Chunk::WIDTH as i64) as usize] = replace_with;
			Some(to_despawn)
		} else {
			None
		}
	}
}

#[derive(Component)]
struct PlayerLoc {
	x: f32,
#[allow(dead_code)]
	y: f32,
}

#[derive(Component)]
struct WalkAnimation {
	frame: usize,
}

#[derive(Component, Deref, DerefMut)]
struct WalkAnimationTimer(Timer);

#[derive(Component)]
struct Character;

#[derive(Component)]
struct UserMovement {
	x_dir: i32,
	sprinting: bool,
	touching_grass: bool,
}

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct Momentum {
	x: f32,
	y: f32,
}

fn update_block(
	mut blocks: Query<(&Tile, &mut Transform, &mut TextureAtlasSprite)>,
	player: Query<&PlayerLoc, With<Character>>,
	query: Query<&Chunk>,
) {
	for chunk in &query {
		for y in 0..Chunk::HEIGHT {
			for x in 0..Chunk::WIDTH {
				if let Some(tile) = chunk.at(x, y) {
					let mut dir_flags: usize = 0;
					if let Some(checking) = chunk.at(x, y + 1) {
						if let Ok((tile, _, _)) = blocks.get(checking) {
							if tile.smooths {
								dir_flags |= 1;
							}
						}
					}
					if let Some(checking) = chunk.at(x, y - 1) {
						if let Ok((tile, _, _)) = blocks.get(checking) {
							if tile.smooths {
								dir_flags |= 2;
							}
						}
					}
					if let Some(checking) = chunk.at(x + 1, y) {
						if let Ok((tile, _, _)) = blocks.get(checking) {
							if tile.smooths {
								dir_flags |= 4;
							}
						}
					}
					if let Some(checking) = chunk.at(x - 1, y) {
						if let Ok((tile, _, _)) = blocks.get(checking) {
							if tile.smooths {
								dir_flags |= 8;
							}
						}
					}
					if let Ok((tile, mut transfem, mut sprite)) = blocks.get_mut(tile) {
						if let Ok(loc) = player.get_single() {
							transfem.translation = Vec3::new(
								((chunk.relative_x * 64 + x) as f32 - loc.x) * 8.0,
								((chunk.relative_y * 64 + y) as f32 - loc.y) * 8.0 - 16.0,
								BLOCK_LAYER
							);
						}
						if tile.smooths {
							sprite.index = dir_flags;
						}
					}
				}
			}
		}
	}
}

const DEFAULT_MOVE_SPEED: f32 = 8.0;

fn move_player(
	time: Res<Time>,
	mut query: Query<
		(
			&mut Momentum,
			&UserMovement
		),
		With<Character>
	>
) {
	let move_distance: f32 = time.delta_seconds() * DEFAULT_MOVE_SPEED;
	for (mut loc, move_state) in &mut query {
		loc.x += move_distance * move_state.x_dir as f32 * (move_state.sprinting as u8 as f32 + 1.0);
	}
}

fn do_momentum(
	time: Res<Time>,
	loaded_chunks: Query<&LoadedChunks>,
	check_collision: Query<&Chunk>,
	tiles: Query<&Tile>,
	mut query: Query<(
		&mut PlayerLoc,
		&mut Momentum,
		&mut UserMovement,
	)>
) {
	let move_x: f32 = time.delta_seconds() * DEFAULT_MOVE_SPEED;
	let move_y: f32 = time.delta_seconds() * DEFAULT_MOVE_SPEED;

	for (mut loc, mut momentum, mut move_state) in &mut query {
		momentum.y -= time.delta_seconds() * 9.81;
		move_state.touching_grass = false;
		if let Ok(loaded) = loaded_chunks.get_single() {
			if let Some(checking) = loaded.at(&check_collision, loc.x.round() as i64, loc.y.round() as i64) {
				if let Ok(tile_check) = tiles.get(checking) {
					if tile_check.smooths {
						if momentum.y <= 0.0 {
							momentum.x -= time.delta_seconds() * momentum.x * 3.0;
							momentum.y = 0.0;
							loc.y = loc.y.round();
							move_state.touching_grass = true;
						}
					}
				}
			}
		}
		loc.x += momentum.x * move_x;
		loc.y += momentum.y * move_y;
	}
}

fn animate_character(
	time: Res<Time>,
	mut query: Query<
		(
			&mut WalkAnimation,
			&mut TextureAtlasSprite, 
			&mut WalkAnimationTimer,
			&mut Transform,
			&UserMovement
		),
		With<Character>
	>
) {
	for (mut walk_animation, mut icon_state, mut walk_timer, mut transform, move_state) in &mut query {
		if move_state.sprinting {
			walk_timer.tick(time.delta() * 2);
		} else {
			walk_timer.tick(time.delta());
		}
		if move_state.x_dir == 0 {
			walk_animation.frame = 0;
			icon_state.index = 0;
		} else {
			if walk_timer.just_finished() {
				icon_state.index = walk_animation.frame;
				walk_animation.frame += 1;
				if walk_animation.frame >= 8 {
					walk_animation.frame = 1;
				}
				if move_state.x_dir > 0 {
					transform.scale = Vec3::new(1.0, 1.0, 0.0);
				} else {
					transform.scale = Vec3::new(-1.0, 1.0, 0.0);
				}
			}
		}
	}
}

fn fps_update_system(
	diagnostics: Res<Diagnostics>,
	mut query: Query<
		&mut Text,
		With<FpsText>
	>
) {
	for mut text in &mut query {
		if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
			if let Some(value) = fps.smoothed() {
				text.sections[1].value = format!("{value:.2}");
			}
		}
	}
}

fn player_input (
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut texture_atlases: ResMut<Assets<TextureAtlas>>,
	keys: Res<Input<KeyCode>>,
	mouse_keys: Res<Input<MouseButton>>,
	loaded_chunks: Query<&LoadedChunks>,
	mut chunks: Query<&mut Chunk>,
	blocks: Query<&Tile>,
	windows: Res<Windows>,
	projection: Query<&PixelProjection>,
	mut query: Query<
		(
			&mut UserMovement,
			&mut Momentum,
			&PlayerLoc,
		),
		With<Character>
	>
) {
	for (mut move_state, mut momentum, loc) in &mut query {
		let mut x_dir: i32 = 0;
		if move_state.touching_grass {
			if keys.pressed(KeyCode::D) {
				x_dir += 1;
			}
			if keys.pressed(KeyCode::A) {
				x_dir -= 1;
			}
		}
		move_state.x_dir = x_dir;
		if keys.pressed(KeyCode::W) {

		}
		if keys.pressed(KeyCode::S) {

		}
		if keys.pressed(KeyCode::LShift) {
			move_state.sprinting = true;
		} else {
			move_state.sprinting = false;
		}
		if keys.pressed(KeyCode::Space) {
			if move_state.touching_grass {
				momentum.y += 3.0;
			}
		}

		let window = windows.get_primary().unwrap();

		if let Some(position) = window.cursor_position() {
			if let Ok(proj) = projection.get_single() {
				let width: f32 = window.width();
				let height: f32 = window.height();
				let play_width: f32 = proj.desired_width.unwrap_or(0) as f32 * proj.zoom as f32 * 0.5;
				let play_height: f32 = proj.desired_height.unwrap_or(0) as f32 * proj.zoom as f32 * 0.5;
				let real_mouse_x: f32 = ((position.x - width * 0.5) / proj.zoom as f32).round();
				let real_mouse_y: f32 = ((position.y - height * 0.5) / proj.zoom as f32).round();
				if real_mouse_x.abs() <= play_width/2.0 && real_mouse_y.abs() <= play_height/2.0 {
					let clicked_block_x: i64 = (loc.x + (real_mouse_x / 8.0)).round() as i64;
					let clicked_block_y: i64 = (loc.y + (real_mouse_y / 8.0) + 2.0).round() as i64;
					if mouse_keys.any_just_pressed([MouseButton::Left, MouseButton::Right]) {
						let loaded = loaded_chunks.get_single().unwrap();
						if let Some(found) = loaded.at_mut(&chunks, clicked_block_x, clicked_block_y) {
							if let Ok(found_block) = blocks.get(found) {
								if !found_block.smooths && mouse_keys.just_pressed(MouseButton::Left) {
									commands.entity(found).despawn();
									let wood_png: Handle<Image> = asset_server.load("Sprites/Blocks/wood.png");
									let wood_atlas: TextureAtlas = TextureAtlas::from_grid(//12x12
										wood_png,
										Vec2::splat(12.0),
										4,
										4,
										None,
										None,
									);
									let wood_spritesheet: Handle<TextureAtlas> = texture_atlases.add(wood_atlas);
									let new_block = commands.spawn((
										SpriteSheetBundle {
											texture_atlas: wood_spritesheet.clone(),
											transform: Transform {
												translation: Vec3 {
													z: BLOCK_LAYER,
													..default()
												},
												..default()
											},
											..default()
										},
										Tile { smooths: true },
									)).id();
									loaded.replace_at(&mut chunks, new_block, clicked_block_x, clicked_block_y);
								} else if found_block.smooths && mouse_keys.just_pressed(MouseButton::Right) {
									commands.entity(found).despawn();
									let air_png: Handle<Image> = asset_server.load("Sprites/Blocks/air.png");
									let air_atlas: TextureAtlas = TextureAtlas::from_grid(//12x12
										air_png,
										Vec2::splat(12.0),
										1,
										1,
										None,
										None,
									);
									let air_spritesheet: Handle<TextureAtlas> = texture_atlases.add(air_atlas);
									let new_block = commands.spawn((
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
										Tile { smooths: false },
									)).id();
									loaded.replace_at(&mut chunks, new_block, clicked_block_x, clicked_block_y);
								}
							}
						}
					}
				}
			}
		}
	}
}

fn setup (
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
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
		Character,
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
		WalkAnimationTimer(Timer::from_seconds(0.05, TimerMode::Repeating)),
		WalkAnimation {
			frame: 0
		},
		PlayerLoc {
			x: 0.0,
			y: 11.0,
		},
		Momentum {
			x: 0.0,
			y: 0.0
		},
		UserMovement {
			x_dir: 0,
			sprinting: false,
			touching_grass: false,
		},
	));

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
		WalkAnimation {
			frame: 0
		},
		FpsText,
	));

	let chunk_data0: [Entity; Chunk::SIZE] = core::array::from_fn(
		|i| match i as i64 / Chunk::WIDTH {
			0..=9 => {
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
						Tile { smooths: true },
				)).id()
			},
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
						Tile { smooths: true },
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
						Tile { smooths: false },
				)).id(),
			_ => panic!(),// This shoould not be possible
		}
	);
	let chunk_data1: [Entity; Chunk::SIZE] = core::array::from_fn(
		|i| match i as i64 / Chunk::WIDTH {
			0..=8 => {
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
						Tile { smooths: true },
				)).id()
			},
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
						Tile { smooths: true },
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
						Tile { smooths: false },
				)).id(),
			_ => panic!(),// This shoould not be possible
		}
	);
	let chunk_data2: [Entity; Chunk::SIZE] = core::array::from_fn(
		|i| match i as i64 / Chunk::WIDTH {
			0..=10 => {
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
						Tile { smooths: true },
				)).id()
			},
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
						Tile { smooths: true },
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
						Tile { smooths: false },
				)).id(),
			_ => panic!(),// This shoould not be possible
		}
	);

	let chunk0 = commands.spawn(
		Chunk::new(
			chunk_data0,
			-1, 0
		)
	).id();
	let chunk1 = commands.spawn(
		Chunk::new(
			chunk_data1,
			0, 0
		)
	).id();
	let chunk2 = commands.spawn(
		Chunk::new(
			chunk_data2,
			1, 0
		)
	).id();

	commands.spawn(
		LoadedChunks::new(vec!(chunk0, chunk1, chunk2))
	);

	commands.spawn(PixelCameraBundle::from_resolution(320, 240));
}
