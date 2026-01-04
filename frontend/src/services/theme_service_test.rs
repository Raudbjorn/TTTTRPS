#[cfg(test)]
mod tests {
    use crate::services::theme_service::*;

    #[test]
    fn test_theme_definition_defaults() {
        let def = ThemeDefinition::default();
        // Default should be fantasy
        assert_eq!(def.text_primary[0], 0.98); // Lightness
        assert!(def.bg_image.contains("radial-gradient"));
    }

    #[test]
    fn test_generate_css_output() {
        let weights = ThemeWeights::preset("fantasy");
        let css = generate_css(&weights);

        // Check for key variables
        assert!(css.contains("--bg-deep:"));
        assert!(css.contains("--bg-image:"));
        assert!(css.contains("radial-gradient"));
    }

    #[test]
    fn test_blend_themes_bg_image_logic() {
        // Create weights where Cosmic is dominant
        let mut weights = ThemeWeights::zeroed();
        weights.fantasy = 0.2;
        weights.cosmic = 0.8;

        let mixed = blend_themes(&weights);

        // Should have cosmic's background image (Starfield)
        assert!(mixed.bg_image.contains("ellipse at bottom"));
    }

    #[test]
    fn test_blend_colors_interpolation() {
        // 50/50 blend between Fantasy and Terminal
        // Fantasy bg_deep L=0.10
        // Terminal bg_deep L=0.05
        // Expected L=0.075
        let mut weights = ThemeWeights::zeroed();
        weights.fantasy = 0.5;
        weights.terminal = 0.5;

        let mixed = blend_themes(&weights);

        // Allow small float error
        assert!((mixed.bg_deep[0] - 0.075).abs() < 0.001);
    }
}
