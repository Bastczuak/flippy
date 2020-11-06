use amethyst::assets::{AssetStorage, Loader};
use amethyst::core::ecs::{
  Builder, Component, DenseVecStorage, Dispatcher, DispatcherBuilder, Entities, Join, NullStorage,
  Read, ReadStorage, System, SystemData, World, WorldExt, Write, WriteStorage,
};
use amethyst::core::math::Vector3;
use amethyst::core::{EventReader, Time, Transform, TransformBundle};
use amethyst::derive::EventReader;
use amethyst::input::{
  is_close_requested, is_key_down, BindingTypes, InputBundle, InputEvent, InputHandler,
  StringBindings, VirtualKeyCode,
};
use amethyst::renderer::types::DefaultBackend;
use amethyst::renderer::{
  Camera, ImageFormat, RenderFlat2D, RenderToWindow, RenderingBundle, SpriteRender, SpriteSheet,
  SpriteSheetFormat, Texture,
};
use amethyst::shrev::{EventChannel, ReaderId};
use amethyst::ui::UiEvent;
use amethyst::utils::application_root_dir;
use amethyst::winit::Event;
use amethyst::{CoreApplication, GameData, GameDataBuilder, State, StateData, Trans};
use rand::rngs::ThreadRng;
use rand::{thread_rng, Rng};

const VIRTUAL_WIDTH: f32 = 512.;
const VIRTUAL_HEIGHT: f32 = 288.;
const GROUND_HEIGHT: f32 = 12.;
const GROUND_WIDTH: f32 = 1100.;
const BACKGROUND_SCROLL_SPEED: f32 = 30.;
const GROUND_SCROLL_SPEED: f32 = 61.;
const BACKGROUND_LOOPING_POINT: f32 = 413.;
const BACKGROUND_LOOPING_OFFSET: f32 = 290.;
const BIRD_GRAVITY: f32 = -26.;
const BIRD_WIDTH: f32 = 38.;
const BIRD_HEIGHT: f32 = 24.;
const BIRD_JUMP: f32 = 4.;
const PIPE_SCROLL: f32 = -60.;
const PIPE_WIDTH: f32 = 70.;
const PIPE_HEIGHT: f32 = 288.;
const PIPE_GAP: f32 = 110.;

#[derive(Debug)]
enum BackgroundType {
  Background,
  Ground,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GameEvent {
  Collision,
}

#[derive(Clone, Debug, EventReader)]
#[reader(MyStateEventReader)]
pub enum MyStateEvent<T = StringBindings>
where
  T: BindingTypes + Clone,
{
  Window(Event),
  Ui(UiEvent),
  Input(InputEvent<T>),
  Game(GameEvent),
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
      transform.prepend_translation_y(bird.dy);
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

struct CollisionSystem;

impl<'a> System<'a> for CollisionSystem {
  type SystemData = (
    ReadStorage<'a, Bird>,
    ReadStorage<'a, Background>,
    ReadStorage<'a, Pipe>,
    ReadStorage<'a, Transform>,
    Write<'a, EventChannel<GameEvent>>,
  );

  fn run(&mut self, (birds, backgrounds, pipes, transforms, mut event_ch): Self::SystemData) {
    for (_, transform) in (&birds, &transforms).join() {
      let bird_x = transform.translation().x;
      let bird_y = transform.translation().y;

      for (_, transform) in (&pipes, &transforms).join() {
        let pipe_x = transform.translation().x - (PIPE_WIDTH / 2.);
        let pipe_y = transform.translation().y - (PIPE_HEIGHT / 2.);

        if point_in_rect(
          bird_x,
          bird_y,
          pipe_x - BIRD_WIDTH / 2.,
          pipe_y - BIRD_HEIGHT / 2.,
          pipe_x + PIPE_WIDTH + BIRD_WIDTH / 2.,
          pipe_y + PIPE_HEIGHT + BIRD_HEIGHT / 2.,
        ) {
          event_ch.single_write(GameEvent::Collision);
        }
      }

      for (background, transform) in (&backgrounds, &transforms).join() {
        match background.b_type {
          BackgroundType::Background => {}
          BackgroundType::Ground => {
            let background_x = transform.translation().y - (GROUND_WIDTH / 2.);
            let background_y = transform.translation().y - (GROUND_HEIGHT / 2.);

            if point_in_rect(
              bird_x,
              bird_y,
              background_x - BIRD_WIDTH / 2.,
              background_y - BIRD_HEIGHT / 2.,
              background_x + GROUND_WIDTH + BIRD_WIDTH / 2.,
              background_y + GROUND_HEIGHT + BIRD_HEIGHT / 2.,
            ) {
              event_ch.single_write(GameEvent::Collision);
            }
          }
        }
      }
    }
  }
}

#[derive(Default)]
struct PlayState<'a, 'b> {
  pipe_spawn_timer: Option<f32>,
  pipe_sprite: Option<SpriteRender>,
  rand: Option<ThreadRng>,
  dispatcher: Option<Dispatcher<'a, 'b>>,
}

impl<'a, 'b> State<GameData<'a, 'b>, MyStateEvent> for PlayState<'a, 'b> {
  fn on_start(&mut self, _data: StateData<'_, GameData<'_, '_>>) {
    let world = _data.world;

    let mut dispatcher_builder = DispatcherBuilder::new();
    dispatcher_builder.add(BackgroundSystem, "background_system", &[]);
    dispatcher_builder.add(BirdSystem, "bird_system", &[]);
    dispatcher_builder.add(PipeSystem, "pipe_system", &[]);
    dispatcher_builder.add(
      CollisionSystem,
      "collision_system",
      &["bird_system", "pipe_system"],
    );
    let mut dispatcher = dispatcher_builder.build();
    dispatcher.setup(world);
    self.dispatcher = Some(dispatcher);

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
    event: MyStateEvent,
  ) -> Trans<GameData<'a, 'b>, MyStateEvent> {
    if let MyStateEvent::Window(event) = &event {
      if is_close_requested(&event) || is_key_down(&event, VirtualKeyCode::Escape) {
        return Trans::Quit;
      }
    }
    if let MyStateEvent::Game(GameEvent::Collision) = event {
      return Trans::Push(Box::new(PauseState));
    }
    Trans::None
  }

  fn update(
    &mut self,
    data: StateData<'_, GameData<'a, 'b>>,
  ) -> Trans<GameData<'a, 'b>, MyStateEvent> {
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

    if let Some(dispatcher) = self.dispatcher.as_mut() {
      dispatcher.dispatch(&data.world);
    }
    data.data.update(&data.world);
    Trans::None
  }
}

#[derive(Default)]
struct PauseState;

impl<'a, 'b> State<GameData<'a, 'b>, MyStateEvent> for PauseState {
  fn handle_event(
    &mut self,
    data: StateData<'_, GameData<'a, 'b>>,
    event: MyStateEvent<StringBindings>,
  ) -> Trans<GameData<'a, 'b>, MyStateEvent<StringBindings>> {
    if let MyStateEvent::Window(event) = &event {
      if is_key_down(&event, VirtualKeyCode::Space) {
        let entities = data.world.entities();
        let pipes = data.world.read_storage::<Pipe>();
        let birds = data.world.read_storage::<Bird>();
        let mut transforms = data.world.write_storage::<Transform>();
        for (_, transform) in (&birds, &mut transforms).join() {
          transform.set_translation(Vector3::new(0., 0., 4.));
        }
        for (e, _) in (&entities, &pipes).join() {
          entities
            .delete(e)
            .expect("Couldn't delete pipe entity while state was paused!");
        }
        Trans::Pop
      } else if is_close_requested(&event) || is_key_down(&event, VirtualKeyCode::Escape) {
        Trans::Quit
      } else {
        Trans::None
      }
    } else {
      Trans::None
    }
  }

  fn update(
    &mut self,
    data: StateData<'_, GameData<'a, 'b>>,
  ) -> Trans<GameData<'a, 'b>, MyStateEvent<StringBindings>> {
    data.data.update(&data.world);
    Trans::None
  }
}

fn point_in_rect(x: f32, y: f32, left: f32, bottom: f32, right: f32, top: f32) -> bool {
  x >= left && x <= right && y >= bottom && y <= top
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

fn main() -> amethyst::Result<()> {
  amethyst::start_logger(Default::default());

  let app_root = application_root_dir()?;
  let display_conf_path = app_root.join("config/display.ron");
  let assets_dir = app_root.join("assets");

  let game_data = GameDataBuilder::default()
    .with_bundle(TransformBundle::new())?
    .with_bundle(InputBundle::<StringBindings>::new())?
    .with_bundle(
      RenderingBundle::<DefaultBackend>::new()
        .with_plugin(
          RenderToWindow::from_config_path(display_conf_path)?.with_clear([0.0, 0.0, 0.0, 1.0]),
        )
        .with_plugin(RenderFlat2D::default()),
    )?;
  let mut game = CoreApplication::<_, MyStateEvent, MyStateEventReader>::build(
    assets_dir,
    PlayState::default(),
  )?
  .build(game_data)?;
  game.run();
  Ok(())
}
