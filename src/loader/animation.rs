use glam::{Mat4, Quat, Vec3, Vec4};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpolation {
    #[default]
    Linear,
    Step,
    CubicSpline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetProperty {
    Translation,
    Rotation, // Quaternion [x, y, z, w]
    Scale,
    Weights, // Morph target weights
}

#[derive(Debug, Clone)]
pub struct AnimationChannel {
    pub target_node: usize,
    pub property: TargetProperty,
    pub interpolation: Interpolation,
    pub timestamps: Vec<f32>,
    pub values: Vec<f32>, // 3 floats per key for T/S, 4 for R, N for weights
}

#[derive(Debug, Clone, Default)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub channels: Vec<AnimationChannel>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SceneNode {
    pub name: String,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub initial_translation: [f32; 3],
    pub initial_rotation: [f32; 4], // Quaternion [x, y, z, w]
    pub initial_scale: [f32; 3],
    pub mesh_index: Option<usize>,
}

impl Default for SceneNode {
    fn default() -> Self {
        Self {
            name: String::new(),
            parent: None,
            children: Vec::new(),
            initial_translation: [0.0, 0.0, 0.0],
            initial_rotation: [0.0, 0.0, 0.0, 1.0],
            initial_scale: [1.0, 1.0, 1.0],
            mesh_index: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Joint {
    pub node_index: usize,
    pub inverse_bind_matrix: [[f32; 4]; 4],
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct Skin {
    pub name: String,
    pub joints: Vec<Joint>,
}

/// Runtime controller managing animation playback, time, speed, clips, and pose evaluation.
#[derive(Debug, Clone)]
pub struct AnimationController {
    pub state: PlaybackState,
    pub current_time: f32,
    pub speed: f32,
    pub is_looping: bool,
    pub active_clip_index: usize,
    pub clips: Vec<AnimationClip>,
}

impl Default for AnimationController {
    fn default() -> Self {
        Self {
            state: PlaybackState::Stopped,
            current_time: 0.0,
            speed: 1.0,
            is_looping: true,
            active_clip_index: 0,
            clips: Vec::new(),
        }
    }
}

impl AnimationController {
    pub fn new(has_animation: bool) -> Self {
        let mut ctrl = Self::default();
        if has_animation {
            ctrl.clips.push(AnimationClip {
                name: "Default".to_string(),
                duration: 1.0,
                channels: Vec::new(),
            });
        }
        ctrl
    }

    pub fn from_clips(clips: Vec<AnimationClip>) -> Self {
        Self {
            state: PlaybackState::Stopped,
            current_time: 0.0,
            speed: 1.0,
            is_looping: true,
            active_clip_index: 0,
            clips,
        }
    }

    pub fn has_animation(&self) -> bool {
        !self.clips.is_empty()
    }

    pub fn set_has_animation(&mut self, has_animation: bool) {
        if !has_animation {
            self.clips.clear();
            self.state = PlaybackState::Stopped;
            self.current_time = 0.0;
        }
    }

    pub fn clips(&self) -> &[AnimationClip] {
        &self.clips
    }

    pub fn active_clip(&self) -> Option<&AnimationClip> {
        self.clips.get(self.active_clip_index)
    }

    pub fn active_clip_name(&self) -> &str {
        self.active_clip()
            .map(|c| if c.name.is_empty() { "Clip" } else { c.name.as_str() })
            .unwrap_or("None")
    }

    pub fn duration(&self) -> f32 {
        self.active_clip().map(|c| c.duration).unwrap_or(0.0)
    }

    #[allow(dead_code)]
    pub fn state(&self) -> PlaybackState {
        self.state
    }

    pub fn is_playing(&self) -> bool {
        self.state == PlaybackState::Playing
    }

    pub fn current_time(&self) -> f32 {
        self.current_time
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.max(0.05);
    }

    pub fn set_looping(&mut self, looping: bool) {
        self.is_looping = looping;
    }

    pub fn select_clip(&mut self, index: usize) {
        if index < self.clips.len() {
            self.active_clip_index = index;
            self.current_time = 0.0;
        }
    }

    pub fn play(&mut self) {
        if !self.has_animation() {
            return;
        }
        self.state = PlaybackState::Playing;
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
        }
    }

    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.current_time = 0.0;
    }

    pub fn toggle_play_pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn seek(&mut self, time: f32) {
        let dur = self.duration();
        if dur > 0.0 {
            self.current_time = time.clamp(0.0, dur);
        } else {
            self.current_time = 0.0;
        }
    }

    /// Advances the timeline by dt. Returns true if the time actually changed.
    pub fn update(&mut self, dt: f32) -> bool {
        if self.state != PlaybackState::Playing || !self.has_animation() {
            return false;
        }

        let dur = self.duration();
        if dur <= 0.0 {
            return false;
        }

        self.current_time += dt * self.speed;

        if self.current_time >= dur {
            if self.is_looping {
                self.current_time %= dur;
            } else {
                self.current_time = dur;
                self.state = PlaybackState::Stopped;
            }
        }

        true
    }

    /// Evaluates the current animation clip pose and deforms the submeshes.
    /// Returns true if submeshes were deformed.
    pub fn evaluate(
        &self,
        nodes: &[SceneNode],
        skins: &[Skin],
        submeshes: &mut [crate::loader::SubMesh],
        is_y_up: bool,
    ) -> bool {
        if nodes.is_empty() || submeshes.is_empty() {
            return false;
        }

        let clip = match self.active_clip() {
            Some(c) => c,
            None => return false,
        };

        let t = self.current_time;

        // 1. Evaluate local node transforms
        let mut local_translations = Vec::with_capacity(nodes.len());
        let mut local_rotations = Vec::with_capacity(nodes.len());
        let mut local_scales = Vec::with_capacity(nodes.len());

        for node in nodes {
            local_translations.push(node.initial_translation);
            local_rotations.push(node.initial_rotation);
            local_scales.push(node.initial_scale);
        }

        for channel in &clip.channels {
            if channel.target_node >= nodes.len() {
                continue;
            }
            let n = channel.target_node;
            match channel.property {
                TargetProperty::Translation => {
                    local_translations[n] = sample_channel_vec3(channel, t, local_translations[n]);
                }
                TargetProperty::Rotation => {
                    local_rotations[n] = sample_channel_quat(channel, t, local_rotations[n]);
                }
                TargetProperty::Scale => {
                    local_scales[n] = sample_channel_vec3(channel, t, local_scales[n]);
                }
                TargetProperty::Weights => {}
            }
        }

        // 2. Compute local and world matrices
        let mut local_matrices = Vec::with_capacity(nodes.len());
        for i in 0..nodes.len() {
            let tr = Vec3::from_slice(&local_translations[i]);
            let rot = Quat::from_xyzw(
                local_rotations[i][0],
                local_rotations[i][1],
                local_rotations[i][2],
                local_rotations[i][3],
            ).normalize();
            let sc = Vec3::from_slice(&local_scales[i]);
            local_matrices.push(Mat4::from_scale_rotation_translation(sc, rot, tr));
        }

        let mut world_matrices = vec![Mat4::IDENTITY; nodes.len()];
        for i in 0..nodes.len() {
            let parent_mat = match nodes[i].parent {
                Some(p) if p < world_matrices.len() => world_matrices[p],
                _ => Mat4::IDENTITY,
            };
            world_matrices[i] = parent_mat * local_matrices[i];
        }

        // 3. Deform submeshes
        let mut modified = false;

        for submesh in submeshes.iter_mut() {
            submesh.ensure_rest_geometry();
            let count = submesh.positions.len();
            if count == 0 || submesh.rest_positions.len() != count {
                continue;
            }

            // A) Skeletal skinning
            if let (Some(ref joint_indices), Some(ref joint_weights)) = (
                submesh.joint_indices.as_ref(),
                submesh.joint_weights.as_ref(),
            ) {
                if let Some(skin) = skins.first() {
                    let palette: Vec<Mat4> = skin
                        .joints
                        .iter()
                        .map(|j| {
                            let joint_world = if j.node_index < world_matrices.len() {
                                world_matrices[j.node_index]
                            } else {
                                Mat4::IDENTITY
                            };
                            let inv_bind = Mat4::from_cols_array_2d(&j.inverse_bind_matrix);
                            joint_world * inv_bind
                        })
                        .collect();

                    let has_normals = submesh.normals.len() == count && submesh.rest_normals.len() == count;

                    for k in 0..count {
                        let p_in = Vec3::from_slice(&submesh.rest_positions[k]).extend(1.0);
                        let indices = joint_indices[k];
                        let weights = joint_weights[k];

                        let mut p_out = Vec4::ZERO;
                        for m in 0..4 {
                            let w = weights[m];
                            if w > 0.0 {
                                let j_idx = indices[m] as usize;
                                if j_idx < palette.len() {
                                    p_out += palette[j_idx] * p_in * w;
                                }
                            }
                        }
                        if is_y_up {
                            submesh.positions[k] = [p_out.x, -p_out.z, p_out.y];
                        } else {
                            submesh.positions[k] = [p_out.x, p_out.y, p_out.z];
                        }

                        if has_normals {
                            let n_in = Vec3::from_slice(&submesh.rest_normals[k]).extend(0.0);
                            let mut n_out = Vec4::ZERO;
                            for m in 0..4 {
                                let w = weights[m];
                                if w > 0.0 {
                                    let j_idx = indices[m] as usize;
                                    if j_idx < palette.len() {
                                        n_out += palette[j_idx] * n_in * w;
                                    }
                                }
                            }
                            let n_norm = Vec3::new(n_out.x, n_out.y, n_out.z).normalize_or_zero();
                            if is_y_up {
                                submesh.normals[k] = [n_norm.x, -n_norm.z, n_norm.y];
                            } else {
                                submesh.normals[k] = [n_norm.x, n_norm.y, n_norm.z];
                            }
                        }
                    }
                    modified = true;
                    continue;
                }
            }

            // B) Rigid node transform
            if let Some(node_idx) = submesh.node_index {
                if node_idx < world_matrices.len() {
                    let node_mat = world_matrices[node_idx];
                    let has_normals = submesh.normals.len() == count && submesh.rest_normals.len() == count;

                    for k in 0..count {
                        let p = node_mat.transform_point3(Vec3::from_slice(&submesh.rest_positions[k]));
                        if is_y_up {
                            submesh.positions[k] = [p.x, -p.z, p.y];
                        } else {
                            submesh.positions[k] = [p.x, p.y, p.z];
                        }

                        if has_normals {
                            let n = node_mat.transform_vector3(Vec3::from_slice(&submesh.rest_normals[k])).normalize_or_zero();
                            if is_y_up {
                                submesh.normals[k] = [n.x, -n.z, n.y];
                            } else {
                                submesh.normals[k] = [n.x, n.y, n.z];
                            }
                        }
                    }
                    modified = true;
                }
            }
        }

        modified
    }
}

fn sample_channel_vec3(channel: &AnimationChannel, time: f32, default: [f32; 3]) -> [f32; 3] {
    if channel.timestamps.is_empty() || channel.values.len() < 3 {
        return default;
    }
    if time <= channel.timestamps[0] || channel.timestamps.len() == 1 {
        return [channel.values[0], channel.values[1], channel.values[2]];
    }
    let last = channel.timestamps.len() - 1;
    if time >= channel.timestamps[last] {
        return [
            channel.values[last * 3],
            channel.values[last * 3 + 1],
            channel.values[last * 3 + 2],
        ];
    }

    let idx = match channel.timestamps.binary_search_by(|t| {
        t.partial_cmp(&time).unwrap_or(std::cmp::Ordering::Equal)
    }) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };
    let next_idx = (idx + 1).min(last);
    let t0 = channel.timestamps[idx];
    let t1 = channel.timestamps[next_idx];
    if (t1 - t0).abs() < 1e-6 {
        return [channel.values[idx * 3], channel.values[idx * 3 + 1], channel.values[idx * 3 + 2]];
    }
    let factor = ((time - t0) / (t1 - t0)).clamp(0.0, 1.0);

    let v0 = Vec3::new(
        channel.values[idx * 3],
        channel.values[idx * 3 + 1],
        channel.values[idx * 3 + 2],
    );
    let v1 = Vec3::new(
        channel.values[next_idx * 3],
        channel.values[next_idx * 3 + 1],
        channel.values[next_idx * 3 + 2],
    );

    match channel.interpolation {
        Interpolation::Step => [v0.x, v0.y, v0.z],
        _ => {
            let res = v0.lerp(v1, factor);
            [res.x, res.y, res.z]
        }
    }
}

fn sample_channel_quat(channel: &AnimationChannel, time: f32, default: [f32; 4]) -> [f32; 4] {
    if channel.timestamps.is_empty() || channel.values.len() < 4 {
        return default;
    }
    if time <= channel.timestamps[0] || channel.timestamps.len() == 1 {
        return [
            channel.values[0],
            channel.values[1],
            channel.values[2],
            channel.values[3],
        ];
    }
    let last = channel.timestamps.len() - 1;
    if time >= channel.timestamps[last] {
        return [
            channel.values[last * 4],
            channel.values[last * 4 + 1],
            channel.values[last * 4 + 2],
            channel.values[last * 4 + 3],
        ];
    }

    let idx = match channel.timestamps.binary_search_by(|t| {
        t.partial_cmp(&time).unwrap_or(std::cmp::Ordering::Equal)
    }) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };
    let next_idx = (idx + 1).min(last);
    let t0 = channel.timestamps[idx];
    let t1 = channel.timestamps[next_idx];
    if (t1 - t0).abs() < 1e-6 {
        return [
            channel.values[idx * 4],
            channel.values[idx * 4 + 1],
            channel.values[idx * 4 + 2],
            channel.values[idx * 4 + 3],
        ];
    }
    let factor = ((time - t0) / (t1 - t0)).clamp(0.0, 1.0);

    let q0 = Quat::from_xyzw(
        channel.values[idx * 4],
        channel.values[idx * 4 + 1],
        channel.values[idx * 4 + 2],
        channel.values[idx * 4 + 3],
    ).normalize();
    let q1 = Quat::from_xyzw(
        channel.values[next_idx * 4],
        channel.values[next_idx * 4 + 1],
        channel.values[next_idx * 4 + 2],
        channel.values[next_idx * 4 + 3],
    ).normalize();

    match channel.interpolation {
        Interpolation::Step => [q0.x, q0.y, q0.z, q0.w],
        _ => {
            let res = q0.slerp(q1, factor);
            [res.x, res.y, res.z, res.w]
        }
    }
}
