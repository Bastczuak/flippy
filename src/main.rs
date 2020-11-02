use amethyst::assets::{AssetStorage, Loader};
use amethyst::core::ecs::{
  Builder, Component, DenseVecStorage, Entities, Join, NullStorage, Read, ReadStorage, System,
  World, WorldExt, WriteStorage,
};
use amethyst::core::math::Vector3;
use amethyst::core::{Time, Transform, TransformBundle};
use amethyst::input::{
  is_close_requested, is_key_down, InputBundle, InputHandler, StringBindings, VirtualKeyCode,
};
use amethyst::renderer::types::DefaultBackend;
use amethyst::renderer::{
  Camera, ImageFormat, RenderFlat2D, RenderToWindow, RenderingBundle, SpriteRender, SpriteSheet,
  SpriteSheetFormat, Texture,
};
use amethyst::utils::application_root_dir;
use amethyst::{
  Application, GameData, GameDataBuilder, SimpleState, SimpleTrans, StateData, StateEvent, Trans,
};
use rand::rngs::ThreadRng;
use rand::{thread_rng, Rng};

const VIRTUAL_WIDTH: f32 = 512.;
const VIRTUAL_HEIGHT: f32 = 288.;
const GROUND_HEIGHT: f32 = 12.;
const BACKGROUND_SCROLL_SPEED: f32 = 30.;
const GROUND_SCROLL_SPEED: f32 = 60.;
const BACKGROUND_LOOPING_POINT: f32 = 413.;
const BACKGROUND_LOOPING_OFFSET: f32 = 290.;
const BIRD_GRAVITY: f32 = -30.;
const BIRD_JUMP: f32 = 5.;
const PIPE_SCROLL: f32 = -60.;
const PIPE_WIDTH: f32 = 70.;
const PIPE_GAP: f32 = 90.;

#[derive(Debug)]
enum BackgroundType {
  Background,
  Ground,
}

#[derive(Debug, Component)]
#[storage(DenseVecStorage)]
struct Background {
  b_type: BackgroundType,
  scroll_pos: f32,
}

#[derive(Debug, Default, Component)]
#[storage(DenseVecStorage)]
struct Bird {
  dy: f32,
}

#[derive(Debug, Default, Component)]
#[storage(NullStorage)]
struct Pipe;

struct BackgroundSystem;

impl<'a> System<'a> for BackgroundSystem {
  type SystemData = (
    WriteStorage<'a, Background>,
    WriteStorage<'a, Transform>,
    Read<'a, Time>,
  );

  fn run(&mut self, (mut backgrounds, mut transforms, time): Self::SystemData) {
    for (background, transform) in (&mut backgrounds, &mut transforms).join() {
      match background.b_type {
        BackgroundType::Background => {
          background.scroll_pos = (background.scroll_pos
            + BACKGROUND_SCROLL_SPEED * time.delta_seconds())
            % BACKGROUND_LOOPING_POINT;
          transform.set_translation_x(BACKGROUND_LOOPING_OFFSET - background.scroll_pos);
        }
        BackgroundType::Ground => {
          background.scroll_pos = (background.scroll_pos
            + GROUND_SCROLL_SPEED * time.delta_seconds())
            % BACKGROUND_LOOPING_POINT;
          transform.set_translation_x(BACKGROUND_LOOPING_OFFSET - background.scroll_pos);
        }
      }
    }
  }
}

struct BirdSystem;

impl<'a> System<'a> for BirdSystem {
  type SystemData = (
    WriteStorage<'a, Bird>,
    WriteStorage<'a, Transform>,
    Read<'a, Time>,
    Read<'a, InputHandler<StringBindings>>,
  );

  fn run(&mut self, (mut birds, mut transforms, time, input): Self::SystemData) {
    for (bird, transform) in (&mut birds, &mut transforms).join() {
      bird.dy += BIRD_GRAVITY * time.delta_seconds();
      if input.key_is_down(VirtualKeyCode::Space) {
        bird.dy = BIRD_JUMP;
      }
      if transform.translation().y > -VIRTUAL_HEIGHT / 2. {
        transform.prepend_translation_y(bird.dy);
      }
    }
  }
}

struct PipeSystem;

impl<'a> System<'a> for PipeSystem {
  type SystemData = (
    Entities<'a>,
    ReadStorage<'a, Pipe>,
    WriteStorage<'a, Transform>,
    Read<'a, Time>,
  );

  fn run(&mut self, (entities, pipes, mut transforms, time): Self::SystemData) {
    for (e, _, transform) in (&entities, &pipes, &mut transforms).join() {
      transform.prepend_translation_x(PIPE_SCROLL * time.delta_seconds());
      if transform.translation().x < VIRTUAL_WIDTH / -2. - PIPE_WIDTH {
        entities
          .delete(e)
          .expect("Error while removing non existing entity! This should never happened!");
      }
    }
  }
}

fn init_camera(world: &mut World) {
  world
    .create_entity()
    .with(Camera::standard_2d(VIRTUAL_WIDTH, VIRTUAL_HEIGHT))
    .with(Transform::from(Vector3::new(0., 0., 10.)))
    .build();
}

fn load_sprite<T>(image: T, ron: T, number: usize, world: &World) -> SpriteRender
where
  T: Into<String>,
{
  let texture_handle = {
    let loader = world.read_resource::<Loader>();
    let texture_storage = world.read_resource::<AssetStorage<Texture>>();
    loader.load(image, ImageFormat::default(), (), &texture_storage)
  };

  let sprite_handle = {
    let loader = world.read_resource::<Loader>();
    let sprite_sheet_store = world.read_resource::<AssetStorage<SpriteSheet>>();
    loader.load(
      ron,
      SpriteSheetFormat(texture_handle),
      (),
      &sprite_sheet_store,
    )
  };

  SpriteRender::new(sprite_handle, number)
}

#[derive(Default)]
struct Flappy {
  pipe_spawn_timer: Option<f32>,
  pipe_sprite: Option<SpriteRender>,
  rand: Option<ThreadRng>,
}

impl SimpleState for Flappy {
  fn on_start(&mut self, _data: StateData<'_, GameData<'_, '_>>) {
    let world = _data.world;
    let background_sprite =
      load_sprite("texture/background.png", "texture/background.ron", 0, world);
    let ground_sprite = load_sprite("texture/ground.png", "texture/ground.ron", 0, world);
    let bird_sprite = load_sprite("texture/bird.png", "texture/bird.ron", 0, world);
    let pipe_sprite = load_sprite("texture/pipe.png", "texture/pipe.ron", 0, world);
    self.pipe_spawn_timer.replace(2.);
    self.pipe_sprite.replace(pipe_sprite);
    self.rand.replace(thread_rng());

    init_camera(world);

    world
      .create_entity()
      .with(Background {
        b_type: BackgroundType::Background,
        scroll_pos: 0.,
      })
      .with(background_sprite)
      .with(Transform::from(Vector3::new(
        BACKGROUND_LOOPING_OFFSET,
        0.,
        0.,
      )))
      .build();
    world
      .create_entity()
      .with(Background {
        b_type: BackgroundType::Ground,
        scroll_pos: 0.,
      })
      .with(ground_sprite)
      .with(Transform::from(Vector3::new(
        BACKGROUND_LOOPING_OFFSET,
        (VIRTUAL_HEIGHT - GROUND_HEIGHT) / -2.,
        2.,
      )))
      .build();
    world
      .create_entity()
      .with(Bird::default())
      .with(bird_sprite)
      .with(Transform::from(Vector3::new(0., 0., 4.)))
      .build();
  }

  fn handle_event(
    &mut self,
    _data: StateData<'_, GameData<'_, '_>>,
    event: StateEvent<StringBindings>,
  ) -> SimpleTrans {
    let StateData { .. } = _data;
    if let StateEvent::Window(event) = &event {
      if is_close_requested(&event) || is_key_down(&event, VirtualKeyCode::Escape) {
        Trans::Quit
      } else {
        Trans::None
      }
    } else {
      Trans::None
    }
  }

  fn update(&mut self, data: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
    if let Some(mut timer) = self.pipe_spawn_timer.take() {
      {
        let time = data.world.fetch::<Time>();
        timer -= time.delta_seconds();
      }
      if timer <= 0.0 {
        if let Some(sprite) = self.pipe_sprite.clone() {
          if let Some(mut rand) = self.rand {
            let random_y = rand.gen_range(-40., 40.);
            data
              .world
              .create_entity()
              .with(Pipe::default())
              .with(sprite.clone())
              .with(Transform::from(Vector3::new(
                  VIRTUAL_WIDTH / 2. + PIPE_WIDTH,
                  -VIRTUAL_HEIGHT / 2. + random_y - PIPE_GAP / 2.,
                  3.,
                )))
              .build();
            data
              .world
              .create_entity()
              .with(Pipe::default())
              .with(sprite)
              .with({
                let mut transform = Transform::from(Vector3::new(
                  VIRTUAL_WIDTH / 2. + PIPE_WIDTH,
                  VIRTUAL_HEIGHT / 2. + random_y + PIPE_GAP / 2.,
                  3.,
                ));
                transform.set_rotation_2d(std::f32::consts::PI);
                transform
              })
              .build();
          }
        }
        self.pipe_spawn_timer.replace(2.);
      } else {
        self.pipe_spawn_timer.replace(timer);
      }
    }
    Trans::None
  }
}

fn main() -> amethyst::Result<()> {
  amethyst::start_logger(Default::default());

  let app_root = application_root_dir()?;
  let display_conf_path = app_root.join("config/display.ron");
  let assets_dir = app_root.join("assets");

  let game_data = GameDataBuilder::default()
    .with(BackgroundSystem, "background_system", &[])
    .with(BirdSystem, "bird_system", &[])
    .with(PipeSystem, "pipe_system", &[])
    .with_bundle(TransformBundle::new())?
    .with_bundle(InputBundle::<StringBindings>::new())?
    .with_bundle(
      RenderingBundle::<DefaultBackend>::new()
        .with_plugin(
          RenderToWindow::from_config_path(display_conf_path)?.with_clear([0.0, 0.0, 0.0, 1.0]),
        )
        .with_plugin(RenderFlat2D::default()),
    )?;

  let mut game = Application::new(assets_dir, Flappy::default(), game_data)?;
  game.run();
  Ok(())
}
