use rand::seq::SliceRandom;
use std::time::SystemTime;

/// Particle for celebration animation
#[derive(Debug, Clone)]
pub struct CelebrationParticle {
    pub x: f64,
    pub y: f64,
    pub vel_x: f64,
    pub vel_y: f64,
    pub symbol: char,
    pub color_index: usize,
    pub age: f64,
    pub max_age: f64,
    pub is_text: bool, // Whether this particle is part of text formation
    pub target_x: f64, // Target position for text particles
    pub target_y: f64,
}

impl CelebrationParticle {
    fn new(x: f64, y: f64) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        Self {
            x,
            y,
            vel_x: rng.gen_range(-3.0..3.0), // Increased velocity for visible movement
            vel_y: rng.gen_range(-4.0..-1.0),
            symbol: *['✨', '🎉', '⭐', '💫', '🌟', '✓', '🎊']
                .choose(&mut rng)
                .unwrap_or(&'✨'),
            color_index: rng.gen_range(0..7),
            age: 0.0,
            max_age: rng.gen_range(2.0..4.0),
            is_text: false,
            target_x: x,
            target_y: y,
        }
    }

    fn new_text_particle(
        x: f64,
        y: f64,
        target_x: f64,
        target_y: f64,
        symbol: char,
        color: usize,
    ) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        Self {
            x,
            y,
            vel_x: (target_x - x) * 1.0, // Increased speed towards target
            vel_y: (target_y - y) * 1.0,
            symbol,
            color_index: color,
            age: 0.0,
            max_age: rng.gen_range(3.0..5.0), // Text particles last longer
            is_text: true,
            target_x,
            target_y,
        }
    }

    fn update(&mut self, dt: f64) -> bool {
        if self.is_text {
            // Text particles move towards target and then stay
            let dist_to_target =
                ((self.target_x - self.x).powi(2) + (self.target_y - self.y).powi(2)).sqrt();
            if dist_to_target > 1.0 {
                self.x += self.vel_x * dt;
                self.y += self.vel_y * dt;
                // Slow down as we approach target
                self.vel_x *= 0.95; // Less aggressive slowdown
                self.vel_y *= 0.95;
            } else {
                // Snap to target and stay there
                self.x = self.target_x;
                self.y = self.target_y;
                self.vel_x = 0.0;
                self.vel_y = 0.0;
            }
        } else {
            // Regular particles with physics
            self.x += self.vel_x * dt;
            self.y += self.vel_y * dt;
            self.vel_y += 15.0 * dt; // Increased gravity for more dramatic fall
        }

        self.age += dt;
        self.age < self.max_age
    }
}

/// Animation state for celebration
#[derive(Debug)]
pub struct CelebrationAnimation {
    pub particles: Vec<CelebrationParticle>,
    pub start_time: SystemTime,
    pub duration: f64, // seconds
    pub is_active: bool,
    pub terminal_width: f64,
    pub terminal_height: f64,
}

impl CelebrationAnimation {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            start_time: SystemTime::now(),
            duration: 3.0, // 3 second celebration
            is_active: false,
            terminal_width: 80.0,
            terminal_height: 24.0,
        }
    }

    pub fn start(&mut self, width: u16, height: u16) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        self.particles.clear();
        self.start_time = SystemTime::now();
        self.is_active = true;
        self.terminal_width = width as f64;
        self.terminal_height = height as f64;

        let center_x = width as f64 / 2.0;
        let center_y = height as f64 / 2.0;

        // Choose a random encouraging word
        let words = [
            "PERFECT!",
            "AMAZING!",
            "EXCELLENT!",
            "FLAWLESS!",
            "SUPERB!",
            "BRILLIANT!",
        ];
        let chosen_word = words.choose(&mut rng).unwrap_or(&"PERFECT!");

        // Create text particles for the chosen word
        self.create_text_particles(chosen_word, center_x, center_y, &mut rng);

        // Add some decorative particles around the text with more spread
        for _ in 0..25 {
            let offset_x = rng.gen_range(-15.0..15.0);
            let offset_y = rng.gen_range(-8.0..8.0);
            self.particles.push(CelebrationParticle::new(
                center_x + offset_x,
                center_y + offset_y,
            ));
        }
    }

    fn create_text_particles(
        &mut self,
        text: &str,
        center_x: f64,
        center_y: f64,
        rng: &mut rand::rngs::ThreadRng,
    ) {
        use rand::Rng;

        let char_width = 2.0; // Space between characters
        let text_width = (text.len() as f64 - 1.0) * char_width;
        let start_x = center_x - text_width / 2.0;

        for (i, ch) in text.chars().enumerate() {
            if ch != ' ' {
                // Skip spaces
                let target_x = start_x + (i as f64 * char_width);
                let target_y = center_y - 2.0; // Position text above center

                // Start particles from random positions around the center and move to form text
                let start_x = center_x + rng.gen_range(-10.0..10.0);
                let start_y = center_y + rng.gen_range(-5.0..5.0);

                // Use bright colors for text
                let color = rng.gen_range(0..7);

                self.particles.push(CelebrationParticle::new_text_particle(
                    start_x, start_y, target_x, target_y, ch, color,
                ));
            }
        }
    }

    pub fn update(&mut self) {
        if !self.is_active {
            return;
        }

        let elapsed = self.start_time.elapsed().unwrap_or_default().as_secs_f64();
        if elapsed >= self.duration {
            self.is_active = false;
            self.particles.clear();
            return;
        }

        let dt = 0.1; // Fixed timestep for animation
        self.particles.retain_mut(|particle| {
            let still_alive = particle.update(dt);

            // Remove decorative particles that fall off screen (text particles should stay)
            if !particle.is_text {
                // Remove if particle is way off screen (allow some buffer for smooth exit)
                let buffer = 5.0;
                let off_screen = particle.y > self.terminal_height + buffer
                    || particle.x < -buffer
                    || particle.x > self.terminal_width + buffer;
                still_alive && !off_screen
            } else {
                still_alive
            }
        });
    }
}

impl Default for CelebrationAnimation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Redirect println! in tests behind RUST_LOG to keep CI output clean
    macro_rules! println {
        ($($arg:tt)*) => {{
            if std::env::var("RUST_LOG").is_ok() {
                eprintln!($($arg)*);
            }
        }}
    }

    #[test]
    fn test_celebration_particle_physics() {
        let mut particle = CelebrationParticle::new(10.0, 10.0);
        let initial_y = particle.y;
        let initial_vel_y = particle.vel_y;

        // Update particle with physics
        let still_alive = particle.update(0.1);

        // Particle should still be alive
        assert!(still_alive);

        // Y position should change due to velocity
        assert_ne!(particle.y, initial_y);

        // Y velocity should increase due to gravity (for non-text particles)
        if !particle.is_text {
            assert!(particle.vel_y > initial_vel_y);
        }
    }

    #[test]
    fn test_text_particle_movement() {
        let mut text_particle = CelebrationParticle::new_text_particle(0.0, 0.0, 10.0, 5.0, 'A', 0);

        // Should be a text particle
        assert!(text_particle.is_text);
        assert_eq!(text_particle.symbol, 'A');
        assert_eq!(text_particle.target_x, 10.0);
        assert_eq!(text_particle.target_y, 5.0);

        // Update several times to move towards target
        for _ in 0..10 {
            text_particle.update(0.1);
        }

        // Should be closer to target
        let distance = ((text_particle.target_x - text_particle.x).powi(2)
            + (text_particle.target_y - text_particle.y).powi(2))
        .sqrt();
        assert!(distance < 5.0); // Should be getting closer
    }

    #[test]
    fn test_celebration_animation_perfect_session() {
        let mut celebration = CelebrationAnimation::new();

        // Should start inactive
        assert!(!celebration.is_active);
        assert!(celebration.particles.is_empty());

        // Start celebration
        celebration.start(80, 24);

        // Celebration should be active
        assert!(celebration.is_active);
        assert!(!celebration.particles.is_empty());

        // Update celebration a few times
        for _ in 0..10 {
            celebration.update();
        }

        // Celebration should still be active (duration is 3 seconds)
        assert!(celebration.is_active);
    }

    #[test]
    fn test_celebration_animation_imperfect_session() {
        let celebration = CelebrationAnimation::new();

        // Should not be active by default
        assert!(!celebration.is_active);
        assert!(celebration.particles.is_empty());
    }

    #[test]
    fn test_encouraging_words_selection() {
        let mut celebration = CelebrationAnimation::new();

        // Start celebration multiple times to test different words
        for _ in 0..10 {
            celebration.start(80, 24);

            // Should have particles
            assert!(!celebration.particles.is_empty());

            // Should have both text and decorative particles
            let has_text_particles = celebration.particles.iter().any(|p| p.is_text);
            let has_decorative_particles = celebration.particles.iter().any(|p| !p.is_text);

            assert!(has_text_particles, "Should have text particles");
            assert!(has_decorative_particles, "Should have decorative particles");
        }
    }

    #[test]
    fn test_celebration_particle_movement() {
        let mut celebration = CelebrationAnimation::new();

        // Start celebration
        celebration.start(80, 24);
        assert!(celebration.is_active);
        assert!(!celebration.particles.is_empty());

        // Record initial positions
        let initial_positions: Vec<(f64, f64)> =
            celebration.particles.iter().map(|p| (p.x, p.y)).collect();

        // Update animation several times
        for _ in 0..5 {
            celebration.update();
        }

        // Check that particles have moved
        let moved_count = celebration
            .particles
            .iter()
            .zip(initial_positions.iter())
            .filter(|(p, &(init_x, init_y))| {
                (p.x - init_x).abs() > 0.1 || (p.y - init_y).abs() > 0.1
            })
            .count();

        assert!(moved_count > 0, "Particles should move after updates");
        println!(
            "✅ {} out of {} particles moved",
            moved_count,
            celebration.particles.len()
        );
    }

    #[test]
    fn test_particles_removed_when_off_screen() {
        let mut celebration = CelebrationAnimation::new();

        // Start celebration with a small terminal size
        celebration.start(20, 10);
        let initial_count = celebration.particles.len();

        // Manually create a particle that's way off screen
        celebration
            .particles
            .push(CelebrationParticle::new(100.0, 100.0)); // Way off screen

        // Update animation - off-screen particles should be removed
        for _ in 0..10 {
            celebration.update();
        }

        // Should have fewer particles now (the off-screen one should be removed)
        assert!(celebration.particles.len() <= initial_count);

        // None of the remaining particles should be way off screen
        for particle in &celebration.particles {
            if !particle.is_text {
                let off_screen = particle.y > 15.0 || particle.x < -5.0 || particle.x > 25.0;
                assert!(
                    !off_screen,
                    "Particle at ({}, {}) should have been removed",
                    particle.x, particle.y
                );
            }
        }

        println!("✅ Off-screen particles properly removed");
    }
}
