
use std::io::prelude::*;

use bevy:: {
	prelude::*,
	dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig},
	text::FontSmoothing,
};
use std::collections::HashMap;
use bevy_framepace::*;

#[allow(unused)]
const BACKGROUND_LAYER: f32 = 0.0;
const BLOCK_LAYER: f32 = 1.0;
const MOB_LAYER: f32 = 2.0;

type TileId = usize;

fn main() {
	App::new()
		.add_plugins((
			DefaultPlugins.set(ImagePlugin::default_nearest()),
			FpsOverlayPlugin {
				config: FpsOverlayConfig {
					text_config: TextFont {
						font_size: 36.0,
						font: default(),
						font_smoothing: FontSmoothing::default(),
						..default()
					},
					text_color: Color::srgb(0.0, 1.0, 0.0),
					refresh_interval: core::time::Duration::from_millis(100),
					enabled: false,
					frame_time_graph_config: FrameTimeGraphConfig {
						enabled: false,
						min_fps: 20.0,
						target_fps: 50.0,
					},
				},
			}
		))
		.add_plugins(FramepacePlugin)

		.insert_resource(ClearColor(Color::srgb(0.3, 0.6, 0.6)))
		.insert_resource(TilesShouldUpdate{ should_update: true })
		.insert_resource(TileChangeQueue {..default()})

		.add_systems(Startup, (setup, init_chunks).chain())
		.add_systems(PreUpdate, (change_tile_sprites, update_tiles).run_if(run_if_tiles_should_update))
		.add_systems(Update, (fps_update_config, player_input, do_physics, walk_animation, update_camera, debug_input))
	.run();
}

#[derive(Resource)]
struct TilesShouldUpdate {
	should_update: bool,
}

#[derive(Resource, Default)]
struct TileChangeQueue {
	queue: Vec<(TileId, bool, isize, isize)>,
}

impl TileChangeQueue {
	fn push(&mut self, to_push: (TileId, bool, isize, isize)) {
		self.queue.push(to_push);
	}
}

fn run_if_tiles_should_update(
	should_update: Res<TilesShouldUpdate>
) -> bool {
	if should_update.should_update {
		true
	} else {
		false
	}
}

fn change_tile_sprites(
	mut to_change: ResMut<TileChangeQueue>,
	tile_ids: Res<TileIds>,
	mut tiles: Query<(&mut Tile, &mut Sprite)>,
	chunks: Query<&Chunk>,
) {
	for (change_to, _, x_pos, y_pos) in &to_change.queue {
		if let Some(entity) = chunks.tile_at(*x_pos, *y_pos) {
		if let Ok((mut tile, mut sprite)) = tiles.get_mut(entity.1) {
			tile.id = *change_to;
			sprite.texture_atlas.as_mut().unwrap().layout = tile_ids.by_id(*change_to).texture.0.clone();
			sprite.image = tile_ids.by_id(*change_to).texture.1.clone();
			sprite.texture_atlas.as_mut().unwrap().index = 0;
		}}
	}
	to_change.queue.clear();
}

fn update_tiles(
	tile_ids: Res<TileIds>,
	chunks: Query<&Chunk>,
	check: Query<&Tile>,
	mut should_update: ResMut<TilesShouldUpdate>,
	mut tiles: Query<(Entity, &Tile, &mut Sprite, &mut Transform)>,
) {
	should_update.should_update = false;
	for (entity, block, mut sprite, mut transfem) in &mut tiles {
//		if !tile_ids.by_tile(block).smooths { continue; }

		if let Some((current_x, current_y)) = chunks.find_tile(entity) {
			sprite.texture_atlas.as_mut().unwrap().index =
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
			transfem.translation.y = current_y as f32 * 8.0;
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
	const BLOCKS: TileId = 9;

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
	fn make_texture(&self, id: TileId) -> Sprite {
		Sprite {
			image: self.tiles[id].texture.1.clone(),
			texture_atlas: Some(TextureAtlas {
				layout: self.tiles[id].texture.0.clone(),
				index: 0,
			}),
			..default()
		}
	}

	#[inline]
	fn make_tile(&self, id: TileId) -> Tile {
		Tile {id: id}
	}

	#[inline]
	fn make_bundle(&self, id: TileId) -> (Tile, Sprite, Transform) {(
		self.make_tile(id),
		self.make_texture(id),
		Transform {translation: Vec3::new(0.0, 0.0, BLOCK_LAYER), ..default()},
	)}
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
struct TileData {
	smooths: bool,
	solid: bool,
	texture: (Handle<TextureAtlasLayout>, Handle<Image>),
}

impl TileData {
	#[inline]
	fn new(smooths: bool, solid: bool, texture: (Handle<TextureAtlasLayout>, Handle<Image>)) -> Self {
		Self {
			smooths: smooths,
			solid: solid,
			texture: texture,
		}
	}
}

#[derive(Component)]
struct Chunk {
	tiles: [(Entity, Entity); Chunk::SIZE],
	foreground_map: HashMap<Entity, usize>,
	background_map: HashMap<Entity, usize>,
	x_pos: isize,
	y_pos: isize,
}

impl Chunk {
	const WIDTH: usize = 64;
	const HEIGHT: usize = 64;
	const SIZE: usize = Self::WIDTH * Self::HEIGHT;

	fn new(tiles: [[(Entity, Entity); Self::WIDTH]; Self::HEIGHT], x_pos: isize, y_pos: isize) -> Self {
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
	#[allow(unused_parens)]
		match (x < Self::WIDTH && y < Self::HEIGHT) {
			true => Some(self.tiles[x + y * Self::WIDTH]),
			false => None,
		}
	}

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
	fn find(&self, to_find: Entity) -> Option<(isize, isize)> {
		match self.foreground_map.get(&to_find) {
			Some(found) => Some(((found % Self::WIDTH) as isize, (found / Self::HEIGHT) as isize)),
			None => match self.background_map.get(&to_find) {
				Some(found) => Some(((found % Self::WIDTH) as isize, (found / Self::HEIGHT) as isize)),
				None => None
			}
		}
	}
}

trait ChunkQuery {
	fn tile_at(&self, absolute_x: isize, absolute_y: isize) -> Option<(Entity, Entity)>;
	fn find_tile(&self, to_find: Entity) -> Option<(isize, isize)>;
}

impl<'w, 's> ChunkQuery for Query<'w, 's, &Chunk> {
	fn tile_at(&self, absolute_x: isize, absolute_y: isize) -> Option<(Entity, Entity)> {
		let chunk_x: isize = absolute_x / Chunk::WIDTH as isize;
		let local_x: usize = absolute_x.cast_unsigned() % Chunk::WIDTH;
		let chunk_y: isize = absolute_y / Chunk::HEIGHT as isize;
		let local_y: usize = absolute_y.cast_unsigned() % Chunk::HEIGHT;
		for chunk in self {
			if chunk.x_pos == chunk_x && chunk.y_pos == chunk_y {
			if let Some(tile) = chunk.at(local_x, local_y) {
				return Some(tile);
			}}
		}
		None
	}

	fn find_tile(&self, to_find: Entity) -> Option<(isize, isize)> {
		for chunk in self {
			if let Some((x, y)) = chunk.find(to_find) {
				return Some((x + chunk.x_pos * 64, y + chunk.y_pos * 64));
			}
		}
		None
	}
}

fn write_chunk(
	chunk: &Chunk,
	tiles: &Query<&Tile>,
	file: std::path::PathBuf,
) {
	let mut saving_to = std::fs::File::create(file).unwrap();

	for entity in chunk.tiles {
		if let Ok(found) = tiles.get(entity.0) {
			match saving_to.write_all(&found.id.to_be_bytes()) {
				Ok(_) => (),
				Err(_) => return,//TODO: HANDLE THIS BETTER
			};
		}
		if let Ok(found) = tiles.get(entity.1) {
			match saving_to.write_all(&found.id.to_be_bytes()) {
				Ok(_) => (),
				Err(_) => return,//TODO: HANDLE THIS BETTER
			};
		}
	}
}

fn replace_chunk(
	tile_change_queue: &mut ResMut<TileChangeQueue>,
	file: std::path::PathBuf,
	x_pos: isize,
	y_pos: isize,
) {
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

	for x in 0..=(Chunk::HEIGHT-1) {
		for y in 0..=(Chunk::WIDTH-1) {
			tile_change_queue.push(
                                (tile_data[((x + (y * Chunk::WIDTH)) * 2) + 1], true,
                                (x as isize) + x_pos*(Chunk::WIDTH as isize),
                                (y as isize) + y_pos*(Chunk::HEIGHT as isize))
                        );
		}
	}
}

fn find_chunk<'s>(
	search_through: &'s Query<&Chunk>,
	x_pos: isize,
	y_pos: isize
) -> Option<&'s Chunk> {
	for chunk in search_through {
		if chunk.x_pos == x_pos && chunk.y_pos == y_pos {
			return Some(chunk);
		}
	}
	None
}

fn debug_input(
	mut update_tiles: ResMut<TilesShouldUpdate>,
	mut tile_update_queue: ResMut<TileChangeQueue>,
	keys: Res<ButtonInput<KeyCode>>,
	chunks: Query<&Chunk>,
	tiles: Query<&Tile>,
) {
	if let Some(chunk) = find_chunk(&chunks, 3, 2) {
		if keys.just_pressed(KeyCode::KeyR) {
			let mut write_path = std::env::current_dir().unwrap();
			write_path.push("save");
			write_path.push(format!("{}-{}.chunk", 3, 2));
			write_chunk(&chunk, &tiles, write_path);
		} else if keys.just_pressed(KeyCode::KeyT) {
			let mut read_path = std::env::current_dir().unwrap();
			read_path.push("save");
			read_path.push(format!("{}-{}.chunk", 3, 2));
			replace_chunk(&mut tile_update_queue, read_path, 3, 2);
			update_tiles.should_update = true;
		}
	}
}

fn update_camera (
	mut cam: Query<&mut Transform, (With<Camera>, Without<Player>)>,
	mut player: Query<(&Mob, &mut Transform), With<Player>>,
) {
	if let Ok(mut transfem) = cam.single_mut() {
		if let Ok((mob, mut player_transfem)) = player.single_mut() {
			player_transfem.translation.x = mob.position.x * 8.0;
			player_transfem.translation.y = mob.position.y * 8.0 + 12.0;
			transfem.translation.x = mob.position.x * 8.0;
			transfem.translation.y = mob.position.y * 8.0;
		}
	}
}

fn player_input (
	windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
	keys: Res<ButtonInput<KeyCode>>,
	mouse_keys: Res<ButtonInput<MouseButton>>,
	mut update_tiles: ResMut<TilesShouldUpdate>,
	mut update_queue: ResMut<TileChangeQueue>,
	pixel_projection: Query<&Projection>,
	chunks: Query<&Chunk>,
	tiles: Query<&Tile>,
	mut query: Query<(&mut Player, &mut Mob)>,
) {
	if let Ok((mut player, mut mob)) = query.single_mut() {
		if keys.pressed(KeyCode::Space) {
			mob.jump_state = JumpState::TryJump;
		} else {
			mob.jump_state = JumpState::None;
		}
		mob.walk_state = WalkState::None;
		if keys.pressed(KeyCode::KeyA) {
			mob.walk_state = WalkState::TryLeft;
		}
		if keys.pressed(KeyCode::KeyD) {
			mob.walk_state = WalkState::TryRight;
		}
		if keys.just_pressed(KeyCode::KeyZ) {
			if player.selected_block > TileIds::AIR + 1 {
				player.selected_block -= 1;
			}
		} else if keys.just_pressed(KeyCode::KeyX) {
			if player.selected_block < TileIds::BLOCKS - 1 {
				player.selected_block += 1;
			}
		}

		let window = windows.single().unwrap();

		if mouse_keys.any_just_pressed([MouseButton::Left, MouseButton::Right]) {
		if let Some(cursor) = window.cursor_position() {
		if let Ok(ortho_projection) = pixel_projection.single() {
			let projection: OrthographicProjection = match ortho_projection {
				Projection::Orthographic(ortho) => ortho.clone(),
				_ => OrthographicProjection::default_2d()
			};
//			let play_width: f32 =
//				projection.area.width() as f32 * 0.5;
//			let play_height: f32 =
//				projection.area.height() as f32 * 0.5;
			let real_mouse_x: f32 =
				((cursor.x - (window.width() * 0.5)) * projection.scale as f32).round();
			let real_mouse_y: f32 =
				-((cursor.y - (window.height() * 0.5)) * projection.scale as f32).round();

//			if real_mouse_x <= play_width && real_mouse_y <= play_height {
				let tile_mouse_x: isize =
					(mob.position.x + (real_mouse_x / 8.0)).round() as isize;
				let tile_mouse_y: isize =
					(mob.position.y + (real_mouse_y / 8.0)).round() as isize;

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
//			}
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
	if let Ok(mut mob) = mob_query.single_mut() {
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
		} + Vec2::new(0.0, -GRAVITY * time.delta_secs() * 8.0);
		let mut new_loc: Vec2 = mob.position + new_velocity * time.delta_secs();
		mob.touching_grass = false;

		for y in 0..=mob.size.y.trunc() as isize {
		for x in 0..=mob.size.x.trunc() as isize {
			let checking_x: isize = x + new_loc.x.trunc() as isize;
			let checking_y: isize = y + new_loc.y.trunc() as isize;
			let mob_x: isize = mob.position.x.trunc() as isize + x;
			let mob_y: isize = mob.position.y.trunc() as isize + y;

			if let Some(tile_colliding) = chunks.tile_at(mob_x, checking_y) {
			if let Ok(tile) = blocks.get(tile_colliding.1) {
			if tile_ids.by_tile(tile).solid {
				new_velocity.y = 0.0;
				new_loc.y = mob.position.y.trunc();
				new_velocity.x -= new_velocity.x * 10.0 * time.delta_secs();
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
			&mut Sprite,
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
				if (sprite.texture_atlas.as_mut().unwrap().index < data.first_walk_frame) || (sprite.texture_atlas.as_mut().unwrap().index >= data.last_walk_frame) {
					sprite.texture_atlas.as_mut().unwrap().index = data.first_walk_frame;
				} else {
					sprite.texture_atlas.as_mut().unwrap().index += 1;
				}
			} else {
				sprite.texture_atlas.as_mut().unwrap().index = data.idle_frame;
			}
		}
	}
}

fn fps_update_config(input: Res<ButtonInput<KeyCode>>, mut overlay: ResMut<FpsOverlayConfig>) {
	if input.just_pressed(KeyCode::KeyP) {
		overlay.enabled = !overlay.enabled;
		overlay.frame_time_graph_config.enabled = !overlay.frame_time_graph_config.enabled;
	}
}

fn setup (
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
	mut settings: ResMut<FramepaceSettings>,
) {
	commands.spawn((
		Camera2d::default(),
		Projection::from(OrthographicProjection {
			scale: 0.5,
			..OrthographicProjection::default_2d()
        	})
	));
	settings.limiter = Limiter::from_framerate(60.0);

	let _: Handle<Image> = asset_server.load("title.png");
	let _: Handle<Image> = asset_server.load("titlebg.png");

	let _: Handle<Image> = asset_server.load("UI/button-down.png");
	let _: Handle<Image> = asset_server.load("UI/button-down2.png");
	let _: Handle<Image> = asset_server.load("UI/button-up.png");
	let _: Handle<Image> = asset_server.load("UI/selector.png");

	let _: Handle<Image> = asset_server.load("Sprites/delete.png");
	let _: Handle<Image> = asset_server.load("Sprites/player_human.png");
	let _: Handle<Image> = asset_server.load("Sprites/player_radlad.png");

/*	let _: Handle<TextureAtlasLayout> = texture_atlases.add(
		TextureAtlasLayout::from_grid(
			asset_server.load("Sprites/player_walk_human.png"),
			Vec2::new(16.0, 32.0),
			8,
			1,
			None,
			None
	));*/
	let lizard_walk_atlas: Handle<TextureAtlasLayout> = texture_atlases.add(
		TextureAtlasLayout::from_grid(
			UVec2::new(16, 32),
			9,
			1,
			None,
			None
		)
	);
/*	let _: Handle<TextureAtlasLayout> = texture_atlases.add(
		TextureAtlasLayout::from_grid(
			asset_server.load("Sprites/player_walk_radlad.png"),
			Vec2::new(16.0, 32.0),
			8,
			1,
			None,
			None
	));*/

	commands.insert_resource(TileIds::new([
		TileData::new(false, false,
			(texture_atlases.add(TextureAtlasLayout::from_grid(UVec2::splat(12),1,1,None,None)), asset_server.load("Sprites/Blocks/air.png"))
		),//TileIds::AIR
		TileData::new(true, true,
			(texture_atlases.add(TextureAtlasLayout::from_grid(UVec2::splat(12),4,4,None,None)), asset_server.load("Sprites/Blocks/dirt.png"))
		),//TileIds::DIRT
		TileData::new(true, true,
			(texture_atlases.add(TextureAtlasLayout::from_grid(UVec2::splat(12),4,4,None,None)), asset_server.load("Sprites/Blocks/grass.png"))
		),//TileIds::GRASS
		TileData::new(true, true,
			(texture_atlases.add(TextureAtlasLayout::from_grid(UVec2::splat(12),4,4,None,None)), asset_server.load("Sprites/Blocks/log.png"))
		),//TileIds::LOG
		TileData::new(true, true,
			(texture_atlases.add(TextureAtlasLayout::from_grid(UVec2::splat(12),4,4,None,None)), asset_server.load("Sprites/Blocks/wood.png"))
		),//TileIds::WOOD
		TileData::new(true, true,
			(texture_atlases.add(TextureAtlasLayout::from_grid(UVec2::splat(12),4,4,None,None)), asset_server.load("Sprites/Blocks/stone.png"))
		),//TileIds::STONE
		TileData::new(true, true,
			(texture_atlases.add(TextureAtlasLayout::from_grid(UVec2::splat(12),4,4,None,None)), asset_server.load("Sprites/Blocks/stonebrick.png"))
		),//TileIds::STONEBRICK
		TileData::new(true, true,
			(texture_atlases.add(TextureAtlasLayout::from_grid(UVec2::splat(12),4,4,None,None)), asset_server.load("Sprites/Blocks/glass.png"))
		),//TileIds::GLASS
		TileData::new(true, true, 
			(texture_atlases.add(TextureAtlasLayout::from_grid(UVec2::splat(12),4,4,None,None)), asset_server.load("Sprites/Blocks/glasspane.png"))
		),//TileIds::GLASSPANE
	]));

	commands.spawn((
		Player::default(),
		Mob {
			..default()
		},
		Sprite {
			image: asset_server.load("Sprites/player_lizard.png"),
			texture_atlas: Some(TextureAtlas {
				layout: lizard_walk_atlas.clone(),
				index: 0,
			}),
			..default()
		},
		Transform {
			translation: Vec3 {
				z: MOB_LAYER,
				..default()
			},
			..default()
		},
		PlayerAnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
		PlayerAnimation::default()
	));
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

	let chunk_data0x2: [[(Entity, Entity); Chunk::WIDTH]; Chunk::HEIGHT] =
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
			chunk_data0x2,
			0, 2
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
