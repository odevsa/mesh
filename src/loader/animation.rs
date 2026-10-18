#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct AnimationTrack {
    pub name: String,
    pub duration: f32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct AnimationData {
    pub tracks: Vec<AnimationTrack>,
}

/// Runtime controller that manages playback state (playing, paused, stopped) and timeline.
#[derive(Debug, Clone, Default)]
pub struct AnimationController {
    state: PlaybackState,
    #[allow(dead_code)]
    current_time: f32,
    #[allow(dead_code)]
    speed: f32,
    has_animation: bool,
}

impl AnimationController {
    pub fn new(has_animation: bool) -> Self {
        Self {
            state: PlaybackState::Stopped,
            current_time: 0.0,
            speed: 1.0,
            has_animation,
        }
    }

    #[allow(dead_code)]
    pub fn has_animation(&self) -> bool {
        self.has_animation
    }

    #[allow(dead_code)]
    pub fn set_has_animation(&mut self, has_animation: bool) {
        self.has_animation = has_animation;
        if !has_animation {
            self.state = PlaybackState::Stopped;
            self.current_time = 0.0;
        }
    }

    #[allow(dead_code)]
    pub fn state(&self) -> PlaybackState {
        self.state
    }

    #[allow(dead_code)]
    pub fn current_time(&self) -> f32 {
        self.current_time
    }

    #[allow(dead_code)]
    pub fn speed(&self) -> f32 {
        self.speed
    }

    #[allow(dead_code)]
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    pub fn play(&mut self) {
        if !self.has_animation {
            println!("[AnimationController] Cannot play: model does not have animations.");
            return;
        }
        self.state = PlaybackState::Playing;
        println!("[AnimationController] Play animation (dummy print)");
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
            println!("[AnimationController] Pause animation (dummy print)");
        } else {
            println!("[AnimationController] Pause animation called while state is {:?}", self.state);
        }
    }

    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.current_time = 0.0;
        println!("[AnimationController] Stop animation (dummy print)");
    }
}
