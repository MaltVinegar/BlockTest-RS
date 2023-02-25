
use std::io::prelude::*;

use bevy:: {
	prelude::*,
	diagnostic::{
		Diagnostics,
		FrameTimeDiagnosticsPlugin
	},
	ecs::schedule::ShouldRun,
};
use bevy_pixel_camera::*;
use std::collections::HashMap;

#[allow(unused)]
const BACKGROUND_LAYER: f32 = 0.0;
const BLOCK_LAYER: f32 = 1.0;
const MOB_LAYER: f32 = 2.0;

type TileId = usize;

fn main() {
    static CHUNKINIT: &str = "chunkinit";

	App::new()
		.insert_resource(ClearColor(Color::rgb(0.3, 0.6, 0.6)))
		.insert_resource(TilesShouldUpdate{ should_update: true })
		.insert_resource(TileChangeQueue {..default()})

		.add_startup_stage_after(
			StartupStage::PostStartup, CHUNKINIT, SystemStage::parallel())

		.add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
		.add_plugin(FrameTimeDiagnosticsPlugin::default())
		.add_plugin(PixelCameraPlugin)

		.add_startup_system(setup)
		.add_startup_system_to_stage(CHUNKINIT, init_chunks)

		.add_system_set(
			SystemSet::new()
				.label("update")
				.with_run_criteria(run_if_tiles_should_update)
				.with_system(change_tile_sprites)
				.with_system(update_tiles.after(change_tile_sprites))
		)
		.add_system_set(
			SystemSet::new()
				.label("main")
				.with_system(fps_update_system)
				.with_system(player_input)
				.with_system(do_physics)
				.with_system(walk_animation)
				.with_system(update_camera)
				.with_system(debug_input)
		)
	.run();
}

#[derive(Resource)]
struct TilesShouldUpdate {
	should_update: bool,
}

#[derive(Resource, Default)]
struct TileChangeQueue {
	queue: Vec<(TileId, bool, usize, usize)>,
}

impl TileChangeQueue {
	fn push(&mut self, to_push: (TileId, bool, usize, usize)) {
		self.queue.push(to_push);
	}
}

fn run_if_tiles_should_update(
	mut should_update: ResMut<TilesShouldUpdate>
) -> ShouldRun {
	if should_update.should_update {
		should_update.should_update = false;
		ShouldRun::Yes
	} else {
		ShouldRun::No
	}
}

fn change_tile_sprites(
	mut to_change: ResMut<TileChangeQueue>,
	tile_ids: Res<TileIds>,
	mut tiles: Query<(&mut Tile, &mut Handle<TextureAtlas>, &mut TextureAtlasSprite)>,
	chunks: Query<&Chunk>,
) {
	for (change_to, is_foreground, x_pos, y_pos) in &to_change.queue {
		if let Some(entity) = chunks.tile_at(*x_pos, *y_pos) {
		if let Ok((mut tile, mut atlas, mut texture)) = tiles.get_mut(entity.1) {
			tile.id = *change_to;
			*atlas = tile_ids.by_id(*change_to).texture.clone();
			texture.index = 0;
		}}
	}
	to_change.queue.clear();
}

fn update_tiles(
	tile_ids: Res<TileIds>,
	chunks: Query<&Chunk>,
	check: Query<&Tile>,
	mut tiles: Query<(Entity, &Tile, &mut TextureAtlasSprite, &mut Transform)>,
) {
	for (entity, block, mut sprite, mut transfem) in &mut tiles {
		if !tile_ids.by_tile(block).smooths { continue; }

		if let Some((current_x, current_y)) = chunks.find_tile(entity) {
			sprite.index =
				match chunks.tile_at(current_x, current_y + 1) {
					Some(checking) =>
					match check.get(checking.1) {
						Ok(tile) => match tile_ids.by_tile(tile).smooths {
							true => 1,
							false => 0,
						},
						_ => 0,
					},  _ => 0,
				} |
				match chunks.tile_at(current_x, current_y - 1) {
					Some(checking) =>
					match check.get(checking.1) {
						Ok(tile) => match tile_ids.by_tile(tile).smooths {
							true => 2,
							false => 0,
						},
						_ => 0,
					},  _ => 0,
				} |
				match chunks.tile_at(current_x + 1, current_y) {
					Some(checking) =>
					match check.get(checking.1) {
						Ok(tile) => match tile_ids.by_tile(tile).smooths {
							true => 4,
							false => 0,
						},
						_ => 0,
					},  _ => 0,
				} |
				match chunks.tile_at(current_x - 1, current_y) {
					Some(checking) =>
					match check.get(checking.1) {
						Ok(tile) => match tile_ids.by_tile(tile).smooths {
							true => 8,
							false => 0,
						},
						_ => 0,
					},  _ => 0,
				};

			transfem.translation.x = current_x as f32 * 8.0;
			transfem.translation.y = current_y as f32 * 8.0 - 12.0;
		}
	}
}

#[derive(Resource)]
struct TileIds {
	tiles: [TileData; Self::BLOCKS],
}

#[allow(unused)]
impl TileIds {
	const AIR: TileId = 0;
	const DIRT: TileId = 1;
	const GRASS: TileId = 2;
	const LOG: TileId = 3;
	const WOOD: TileId = 4;
	const STONE: TileId = 5;
	const STONEBRICK: TileId = 6;
	const GLASS: TileId = 7;
	const GLASSPANE: TileId = 8;
	const BLOCKS: usize = 9;

	#[inline]
	fn new(tiles: [TileData; Self::BLOCKS]) -> Self {
		Self {
			tiles: tiles
		}
	}

	#[inline]
	fn by_id(&self, id: TileId) -> &TileData {
		&self.tiles[id]
	}

	#[inline]
	fn by_tile(&self, tile: &Tile) -> &TileData {
		&self.tiles[tile.id]
	}

	#[inline]
	fn make_texture(&self, id: TileId) -> SpriteSheetBundle {
		SpriteSheetBundle {
			texture_atlas: self.tiles[id].texture.clone(),
			transform: Transform {
				translation: Vec3::new(0.0, 0.0, BLOCK_LAYER),
				..default()
			},
			sprite: TextureAtlasSprite {
				..default()
			},
			..default()
		}
	}

	#[inline]
	fn make_tile(&self, id: TileId) -> Tile {
		Tile {id: id}
	}

	#[inline]
	fn make_bundle(&self, id: TileId) -> (Tile, SpriteSheetBundle) {
		(self.make_tile(id), self.make_texture(id))
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

#[derive(Component)]
struct Player {
	selected_block: TileId,
}

impl<'a> Default for Player {
	fn default() -> Self {
		Player {
			selected_block: TileIds::DIRT
		}
	}
}

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
	id: TileId,
}

#[derive(Component)]
struct Foreground;

#[derive(Component)]
struct Background;

#[derive(Component)]
struct TileData {
	smooths: bool,
	solid: bool,
	texture: Handle<TextureAtlas>,
}

impl TileData {
	#[inline]
	fn new(smooths: bool, solid: bool, texture: Handle<TextureAtlas>) -> Self {
		Self {
			smooths: smooths,
			solid: solid,
			texture: texture,
		}
	}
}

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct Chunk {
	tiles: [(Entity, Entity); Chunk::SIZE],
	foreground_map: HashMap<Entity, usize>,
	background_map: HashMap<Entity, usize>,
	x_pos: usize,
	y_pos: usize,
}

impl Chunk {
	const WIDTH: usize = 64;
	const HEIGHT: usize = 64;
	const SIZE: usize = Self::WIDTH * Self::HEIGHT;

	fn new(tiles: [[(Entity, Entity); Self::WIDTH]; Self::HEIGHT], x_pos: usize, y_pos: usize) -> Self {
		Self {
			tiles: core::array::from_fn(
				|i| tiles[i / Self::HEIGHT][i % Self::WIDTH]
			),
			foreground_map: HashMap::new(),
			background_map: HashMap::new(),
			x_pos,
			y_pos,
		}.update_hashmap()
	}

	#[inline]
	fn at(&self, x: usize, y: usize) -> Option<(Entity, Entity)> {
		match x < Self::WIDTH && y < Self::HEIGHT {
			true => Some(self.tiles[x + y * Self::WIDTH]),
			false => None,
		}
	}

	#[allow(unused)]
	#[inline]
	fn update_hashmap(mut self) -> Self {
		self.foreground_map = 
			|tiles: &[(Entity, Entity); Self::SIZE]| -> HashMap<Entity, usize> {
				let mut rv: HashMap<Entity, usize> = HashMap::<Entity, usize>::new();
				for i in 0..tiles.len() {
					rv.insert(tiles[i].1, i);
				}
				rv
			}(&self.tiles);
		self.background_map = 
			|tiles: &[(Entity, Entity); Self::SIZE]| -> HashMap<Entity, usize> {
				let mut rv: HashMap<Entity, usize> = HashMap::<Entity, usize>::new();
				for i in 0..tiles.len() {
					rv.insert(tiles[i].0, i);
				}
				rv
			}(&self.tiles);
		return self;
	}

	#[inline]
	fn find(&self, to_find: Entity) -> Option<(usize, usize)> {
		match self.foreground_map.get(&to_find) {
			Some(found) => Some((found % Self::WIDTH, found / Self::HEIGHT)),
			None => match self.background_map.get(&to_find) {
				Some(found) => Some((found % Self::WIDTH, found / Self::HEIGHT)),
				None => None
			}
		}
	}
}

trait ChunkQuery {
	fn tile_at(&self, absolute_x: usize, absolute_y: usize) -> Option<(Entity, Entity)>;
	fn find_tile(&self, to_find: Entity) -> Option<(usize, usize)>;
}

impl<'w, 's> ChunkQuery for Query<'w, 's, &Chunk> {
	fn tile_at(&self, absolute_x: usize, absolute_y: usize) -> Option<(Entity, Entity)> {
		let chunk_x: usize = absolute_x / Chunk::WIDTH;
		let local_x: usize = absolute_x % Chunk::WIDTH;
		let chunk_y: usize = absolute_y / Chunk::WIDTH;
		let local_y: usize = absolute_y % Chunk::HEIGHT;
		for chunk in self {
			if chunk.x_pos == chunk_x && chunk.y_pos == chunk_y {
			if let Some(tile) = chunk.at(local_x, local_y) {
				return Some(tile);
			}}
		}
		None
	}

	fn find_tile(&self, to_find: Entity) -> Option<(usize, usize)> {
		for chunk in self {
			if let Some((x, y)) = chunk.find(to_find) {
				return Some((x + chunk.x_pos * 64, y + chunk.y_pos * 64));
			}
		}
		None
	}
}

fn write_chunk(chunk: (Entity, &Chunk), commands: &mut Commands, tiles: &Query<&Tile>) {
	let mut save_path = std::env::current_dir().unwrap();
	save_path.push("save");
	save_path.push(format!("{}-{}.chunk", chunk.1.x_pos, chunk.1.y_pos));
	let mut saving_to = std::fs::File::create(save_path).unwrap();

	for entity in chunk.1.tiles {
		if let Ok(found) = tiles.get(entity.1) {
			match saving_to.write_all(&found.id.to_be_bytes()) {
				Ok(_) => (),
				Err(_) => return,//TODO: HANDLE THIS BETTER
			};
			commands.entity(entity.1).despawn();
		}
	}
	commands.entity(chunk.0).despawn();
}

fn read_chunk(
	commands: &mut Commands,
	tile_ids: &Res<TileIds>,
	file: std::path::PathBuf,
	x_pos: usize,
	y_pos: usize
) -> Result<Entity, &'static str> {
	if !std::path::Path::new(&file).exists() {
		Err("File doesn't exist")
	} else {
		let mut reading = std::fs::File::open(file).unwrap();

		let tile_data: [usize; Chunk::SIZE * 2] = core::array::from_fn(
			|_| -> usize {
				let mut data: [u8; std::mem::size_of::<usize>()] =
					[0; std::mem::size_of::<usize>()];
				match reading.read(&mut data) {
					Ok(_) => (),
					Err(_) => panic!(),
				}
				return usize::from_be_bytes(data);
			}
		);

		let tile_entities: [[(Entity, Entity); Chunk::WIDTH]; Chunk::HEIGHT] = 
		core::array::from_fn( |y| -> [(Entity, Entity); Chunk::WIDTH] {
		core::array::from_fn( |x| -> (Entity, Entity) {
			(commands.spawn(
				tile_ids.make_bundle(tile_data[x + y * Chunk::WIDTH * 2])
			).id(),
			commands.spawn(
				tile_ids.make_bundle(tile_data[x + y * Chunk::WIDTH * 2 + 1])
			).id())
		})});

		Ok(commands.spawn(
			Chunk::new(
				tile_entities,
				x_pos, y_pos
			)
		).id())
	}
}

fn find_chunk<'s>(
	search_through: &'s Query<(Entity, &Chunk)>,
	x_pos: usize,
	y_pos: usize
) -> Option<(Entity, &'s Chunk)> {
	for chunk in search_through {
		if chunk.1.x_pos == x_pos && chunk.1.y_pos == y_pos {
			return Some((chunk.0, chunk.1));
		}
	}
	None
}

fn debug_input(
	mut commands: Commands,
	mut update_tiles: ResMut<TilesShouldUpdate>,
	keys: Res<Input<KeyCode>>,
	tile_ids: Res<TileIds>,
	chunks: Query<(Entity, &Chunk)>,
	tiles: Query<&Tile>,
) {
	//3, 2
	if let Some(chunk) = find_chunk(&chunks, 3, 2) {
		if keys.just_pressed(KeyCode::R) {
			write_chunk((chunk.0, chunk.1), &mut commands, &tiles);
			update_tiles.should_update = true;
		}
	} else if keys.just_pressed(KeyCode::T) {
		let mut read_path = std::env::current_dir().unwrap();
		read_path.push("save");
		read_path.push(format!("{}-{}.chunk", 3, 2));
		let _ = read_chunk(&mut commands, &tile_ids, read_path, 3, 2);
		update_tiles.should_update = true;
	}
}

fn update_camera (
	mut cam: Query<&mut Transform, (With<Camera>, Without<Player>)>,
	mut player: Query<(&Mob, &mut Transform), With<Player>>,
) {
	if let Ok(mut transfem) = cam.get_single_mut() {
		if let Ok((mob, mut player_transfem)) = player.get_single_mut() {
			player_transfem.translation.x = mob.position.x * 8.0;
			player_transfem.translation.y = mob.position.y * 8.0;
			transfem.translation.x = mob.position.x * 8.0;
			transfem.translation.y = mob.position.y * 8.0;
		}
	}
}

fn player_input (
	windows: Res<Windows>,
	keys: Res<Input<KeyCode>>,
	mouse_keys: Res<Input<MouseButton>>,
	mut update_tiles: ResMut<TilesShouldUpdate>,
	mut update_queue: ResMut<TileChangeQueue>,
	pixel_projection: Query<&PixelProjection>,
	chunks: Query<&Chunk>,
	tiles: Query<&Tile>,
	mut query: Query<(&mut Player, &mut Mob)>,
) {
	if let Ok((mut player, mut mob)) = query.get_single_mut() {
		if keys.pressed(KeyCode::Space) {
			mob.jump_state = JumpState::TryJump;
		} else {
			mob.jump_state = JumpState::None;
		}
		mob.walk_state = WalkState::None;
		if keys.pressed(KeyCode::A) {
			mob.walk_state = WalkState::TryLeft;
		}
		if keys.pressed(KeyCode::D) {
			mob.walk_state = WalkState::TryRight;
		}
		if keys.just_pressed(KeyCode::Z) {
			if player.selected_block > TileIds::AIR + 1 {
				player.selected_block -= 1;
			}
		} else if keys.just_pressed(KeyCode::X) {
			if player.selected_block < TileIds::BLOCKS - 1 {
				player.selected_block += 1;
			}
		}

		let window = windows.get_primary().unwrap();

		if mouse_keys.any_just_pressed([MouseButton::Left, MouseButton::Right]) {
		if let Some(cursor) = window.cursor_position() {
		if let Ok(projection) = pixel_projection.get_single() {
			let play_width: f32 =
				projection.desired_width.unwrap_or(0) as f32 * 0.5;
			let play_height: f32 =
				projection.desired_height.unwrap_or(0) as f32 * 0.5;
			let real_mouse_x: f32 =
				((cursor.x - window.width() * 0.5) / projection.zoom as f32).round();
			let real_mouse_y: f32 =
				((cursor.y - window.height() * 0.5) / projection.zoom as f32).round();


			if real_mouse_x <= play_width && real_mouse_y <= play_height {
				let tile_mouse_x: usize =
					(mob.position.x + (real_mouse_x / 8.0)).round() as usize;
				let tile_mouse_y: usize =
					(mob.position.y + (real_mouse_y + 12.0) / 8.0).round() as usize;
				if mouse_keys.just_pressed(MouseButton::Left) {
				if let Some(clicking_entity) = chunks.tile_at(tile_mouse_x, tile_mouse_y) {
				if let Ok(clicked_tile) = tiles.get(clicking_entity.1) {
				if clicked_tile.id == TileIds::AIR {
					update_queue.push((
						player.selected_block,
						true,
						tile_mouse_x,
						tile_mouse_y
					));
					update_tiles.should_update = true;
				}}}}
				if mouse_keys.just_pressed(MouseButton::Right) {
				if let Some(clicking_entity) = chunks.tile_at(tile_mouse_x, tile_mouse_y) {
				if let Ok(clicked_tile) = tiles.get(clicking_entity.1) {
				if clicked_tile.id != TileIds::AIR {
					update_queue.push((
						TileIds::AIR,
						true,
						tile_mouse_x,
						tile_mouse_y
					));
					update_tiles.should_update = true;
				}}}}
			}
		}}}
	}
}

const GRAVITY: f32 = 9.81;

fn do_physics(
	time: Res<Time>,
	tile_ids: Res<TileIds>,
	mut mob_query: Query<&mut Mob>,
	chunks: Query<&Chunk>,
	blocks: Query<&Tile>,
) {
	if let Ok(mut mob) = mob_query.get_single_mut() {
		let mut new_velocity = mob.velocity;
		new_velocity += if mob.touching_grass {(
			match mob.jump_state {
				JumpState::TryJump => Vec2::new(0.0, 30.0),
				_ => Vec2::ZERO,
			} +
			match mob.walk_state {
				WalkState::None => Vec2::ZERO,
				_ => Vec2::new(mob.walk_state as i32 as f32 * 5.0, 0.0),
			}
		)} else {
			Vec2::ZERO
		} + Vec2::new(0.0, -GRAVITY * time.delta_seconds() * 8.0);
		let mut new_loc: Vec2 = mob.position + new_velocity * time.delta_seconds();
		mob.touching_grass = false;

		for y in 0..=mob.size.y.trunc() as usize {
		for x in 0..=mob.size.x.trunc() as usize {
			let checking_x: usize = x + new_loc.x.trunc() as usize;
			let checking_y: usize = y + new_loc.y.trunc() as usize;
			let mob_x: usize = mob.position.x.trunc() as usize + x;
			let mob_y: usize = mob.position.y.trunc() as usize + y;

			if let Some(tile_colliding) = chunks.tile_at(mob_x, checking_y) {
			if let Ok(tile) = blocks.get(tile_colliding.1) {
			if tile_ids.by_tile(tile).solid {
				new_velocity.y = 0.0;
				new_loc.y = mob.position.y.trunc();
				new_velocity.x -= new_velocity.x * 10.0 * time.delta_seconds();
				mob.touching_grass = true;
			}}}
			if let Some(tile_colliding) = chunks.tile_at(checking_x, mob_y) {
			if let Ok(tile) = blocks.get(tile_colliding.1) {
			if tile_ids.by_tile(tile).solid {
				new_velocity.x = 0.0;
				new_loc.x = mob.position.x;
			}}}
			else if let Some(tile_colliding) = chunks.tile_at(checking_x, checking_y) {
			if let Ok(tile) = blocks.get(tile_colliding.1) {
			if tile_ids.by_tile(tile).solid {
				new_velocity.x = 0.0;
				new_loc.x = mob.position.x;
			}}}
		}}

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

#[allow(unused)]
fn fps_update_system(
	diagnostics: Res<Diagnostics>,
	player_query: Query<&Mob, With<Player>>,
	mut query: Query<
		&mut Text,
		With<FpsText>
	>
) {
	for mut text in &mut query {
/*		if let Ok(player) = player_query.get_single() {
			text.sections[1].value = format!("{:.2}:{:.2}:{}", player.position.x, player.position.y, player.touching_grass)
		}*/
		if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
			if let Some(value) = fps.smoothed() {
				text.sections[1].value = format!("{value:.2}");
			}
		}
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
	let _: Handle<Image> = asset_server.load("Sprites/player_radlad.png");

	let _: Handle<TextureAtlas> = texture_atlases.add(
		TextureAtlas::from_grid(
			asset_server.load("Sprites/player_walk_human.png"),
			Vec2::new(16.0, 32.0),
			8,
			1,
			None,
			None
	));
	let lizard_walk_atlas: Handle<TextureAtlas> = texture_atlases.add(
		TextureAtlas::from_grid(
			asset_server.load("Sprites/player_lizard.png"),
			Vec2::new(16.0, 32.0),
			9,
			1,
			None,
			None
	));
	let _: Handle<TextureAtlas> = texture_atlases.add(
		TextureAtlas::from_grid(
			asset_server.load("Sprites/player_walk_radlad.png"),
			Vec2::new(16.0, 32.0),
			8,
			1,
			None,
			None
	));

	commands.insert_resource(TileIds::new([
		TileData::new(false, false,
			texture_atlases.add(TextureAtlas::from_grid(
				asset_server.load("Sprites/Blocks/air.png"),
				Vec2::splat(12.0),
				1,
				1,
				None,
				None,
			))),//TileIds::AIR
		TileData::new(true, true,
			texture_atlases.add(TextureAtlas::from_grid(
				asset_server.load("Sprites/Blocks/dirt.png"),
				Vec2::splat(12.0),
				4,
				4,
				None,
				None,
			))),//TileIds::DIRT
		TileData::new(true, true,
			texture_atlases.add(TextureAtlas::from_grid(
				asset_server.load("Sprites/Blocks/grass.png"),
				Vec2::splat(12.0),
				4,
				4,
				None,
				None,
			))),//TileIds::GRASS
		TileData::new(true, true,
			texture_atlases.add(TextureAtlas::from_grid(
				asset_server.load("Sprites/Blocks/log.png"),
				Vec2::splat(12.0),
				4,
				4,
				None,
				None,
			))),//TileIds::LOG
		TileData::new(true, true,
			texture_atlases.add(TextureAtlas::from_grid(
				asset_server.load("Sprites/Blocks/wood.png"),
				Vec2::splat(12.0),
				4,
				4,
				None,
				None,
			))),//TileIds::WOOD
		TileData::new(true, true,
			texture_atlases.add(TextureAtlas::from_grid(
				asset_server.load("Sprites/Blocks/stone.png"),
				Vec2::splat(12.0),
				4,
				4,
				None,
				None,
			))),//TileIds::STONE
		TileData::new(true, true,
			texture_atlases.add(TextureAtlas::from_grid(
				asset_server.load("Sprites/Blocks/stonebrick.png"),
				Vec2::splat(12.0),
				4,
				4,
				None,
				None,
			))),//TileIds::STONEBRICK
		TileData::new(true, true,
			texture_atlases.add(TextureAtlas::from_grid(
				asset_server.load("Sprites/Blocks/glass.png"),
				Vec2::splat(12.0),
				4,
				4,
				None,
				None,
			))),//TileIds::GLASS
		TileData::new(true, true,
			texture_atlases.add(TextureAtlas::from_grid(
				asset_server.load("Sprites/Blocks/glasspane.png"),
				Vec2::splat(12.0),
				4,
				4,
				None,
				None,
			))),//TileIds::GLASSPANE
	]));

	commands.spawn((
		Player::default(),
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
		PlayerAnimation::default()
	));

	commands.spawn(PixelCameraBundle::from_resolution(320, 240));
}

fn init_chunks(
	mut commands: Commands,
	tile_ids: Res<TileIds>,
) {
	let chunk_data0: [[(Entity, Entity); Chunk::WIDTH]; Chunk::HEIGHT] =
	core::array::from_fn( |_| -> [(Entity, Entity); Chunk::WIDTH] {
	core::array::from_fn( |_| -> (Entity, Entity) {(
			commands.spawn(tile_ids.make_bundle(TileIds::AIR)).id(),
			commands.spawn(tile_ids.make_bundle(TileIds::DIRT)).id(),
	)})});
	commands.spawn(
		Chunk::new(
			chunk_data0,
			1, 1
		)
	);

	let chunk_data1: [[(Entity, Entity); Chunk::WIDTH]; Chunk::HEIGHT] =
	core::array::from_fn( |_| -> [(Entity, Entity); Chunk::WIDTH] {
	core::array::from_fn( |_| -> (Entity, Entity) {(
		commands.spawn(tile_ids.make_bundle(TileIds::AIR)).id(),
		commands.spawn(tile_ids.make_bundle(TileIds::DIRT)).id(),
	)})});
	commands.spawn(
		Chunk::new(
			chunk_data1,
			2, 1
		)
	);

	let chunk_data2: [[(Entity, Entity); Chunk::WIDTH]; Chunk::HEIGHT] =
	core::array::from_fn( |_| -> [(Entity, Entity); Chunk::WIDTH] {
	core::array::from_fn( |_| -> (Entity, Entity) {(
		commands.spawn(tile_ids.make_bundle(TileIds::AIR)).id(),
		commands.spawn(tile_ids.make_bundle(TileIds::DIRT)).id(),
	)})});
	commands.spawn(
		Chunk::new(
			chunk_data2,
			3, 1
		)
	);

	let chunk_data3: [[(Entity, Entity); Chunk::WIDTH]; Chunk::HEIGHT] =
	core::array::from_fn( |y: usize| -> [(Entity, Entity); Chunk::WIDTH] {
		core::array::from_fn( |_| -> (Entity, Entity) {(
			commands.spawn(tile_ids.make_bundle(TileIds::AIR)).id(),
			commands.spawn(
				tile_ids.make_bundle(
					match y {
						0..=9 => TileIds::DIRT,
						10 => TileIds::GRASS,
						_ => TileIds::AIR,
			})).id(),
		)})
	});
	commands.spawn(
		Chunk::new(
			chunk_data3,
			1, 2
		)
	);

	let chunk_data4: [[(Entity, Entity); Chunk::WIDTH]; Chunk::HEIGHT] =
	core::array::from_fn( |y: usize| -> [(Entity, Entity); Chunk::WIDTH] {
		core::array::from_fn( |_| -> (Entity, Entity) {(
			commands.spawn(tile_ids.make_bundle(TileIds::AIR)).id(),
			commands.spawn(
				tile_ids.make_bundle(
					match y {
						0..=8 => TileIds::DIRT,
						9 => TileIds::GRASS,
						_ => TileIds::AIR,
					})).id(),
		)})
	});
	commands.spawn(
		Chunk::new(
			chunk_data4,
			2, 2
		)
	);

	let chunk_data5: [[(Entity, Entity); Chunk::WIDTH]; Chunk::HEIGHT] =
	core::array::from_fn( |y: usize| -> [(Entity, Entity); Chunk::WIDTH] {
		core::array::from_fn( |_| -> (Entity, Entity) {(
			commands.spawn(tile_ids.make_bundle(TileIds::AIR)).id(),
			commands.spawn(
				tile_ids.make_bundle(
					match y {
						0..=10 => TileIds::DIRT,
						11 => TileIds::GRASS,
						_ => TileIds::AIR,
					})).id(),
		)})
	});
	commands.spawn(
		Chunk::new(
			chunk_data5,
			3, 2
		)
	);

	let chunk_data6: [[(Entity, Entity); Chunk::WIDTH]; Chunk::HEIGHT] =
	core::array::from_fn( |_| -> [(Entity, Entity); Chunk::WIDTH] {
	core::array::from_fn( |_| -> (Entity, Entity) {(
		commands.spawn(tile_ids.make_bundle(TileIds::AIR)).id(),
		commands.spawn(tile_ids.make_bundle(TileIds::AIR)).id(),
	)})});
	commands.spawn(
		Chunk::new(
			chunk_data6,
			1, 3
		)
	);

	let chunk_data7: [[(Entity, Entity); Chunk::WIDTH]; Chunk::HEIGHT] =
	core::array::from_fn( |_| -> [(Entity, Entity); Chunk::WIDTH] {
	core::array::from_fn( |_| -> (Entity, Entity) {(
		commands.spawn(tile_ids.make_bundle(TileIds::AIR)).id(),
		commands.spawn(tile_ids.make_bundle(TileIds::AIR)).id(),
	)})});
	commands.spawn(
		Chunk::new(
			chunk_data7,
			2, 3
		)
	);

	let chunk_data8: [[(Entity, Entity); Chunk::WIDTH]; Chunk::HEIGHT] =
	core::array::from_fn( |_| -> [(Entity, Entity); Chunk::WIDTH] {
	core::array::from_fn( |_| -> (Entity, Entity) {(
		commands.spawn(tile_ids.make_bundle(TileIds::AIR)).id(),
		commands.spawn(tile_ids.make_bundle(TileIds::AIR)).id(),
	)})});
	commands.spawn(
		Chunk::new(
			chunk_data8,
			3, 3
		)
	);
}
