#[cfg(test)]
mod tests {
    use crate::components::design_system::button::{ButtonVariant, ButtonSize};

    #[test]
    fn test_button_variant_classes() {
        // Just verify that the methods exist and return distinct strings
        let primary = ButtonVariant::Primary.class();
        let destructive = ButtonVariant::Destructive.class();
        assert_ne!(primary, destructive);
        assert!(destructive.contains("bg-red-500"));
    }

    #[test]
    fn test_button_size_classes() {
        let default = ButtonSize::Default.class();
        let sm = ButtonSize::Sm.class();
        assert_ne!(default, sm);
        assert!(sm.contains("text-xs"));
    }
}
