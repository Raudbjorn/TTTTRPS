//! Design System Component Tests
//!
//! Unit tests for design system enums, variants, and styling logic.

use crate::components::design_system::button::{ButtonVariant, ButtonSize};
use crate::components::design_system::badge::BadgeVariant;

// ========================================================================
// ButtonVariant Tests
// ========================================================================

#[test]
fn test_button_variant_default() {
    assert_eq!(ButtonVariant::default(), ButtonVariant::Primary);
}

#[test]
fn test_button_variant_equality() {
    assert_eq!(ButtonVariant::Primary, ButtonVariant::Primary);
    assert_eq!(ButtonVariant::Secondary, ButtonVariant::Secondary);
    assert_eq!(ButtonVariant::Destructive, ButtonVariant::Destructive);
    assert_eq!(ButtonVariant::Ghost, ButtonVariant::Ghost);
    assert_eq!(ButtonVariant::Outline, ButtonVariant::Outline);
    assert_eq!(ButtonVariant::Link, ButtonVariant::Link);

    assert_ne!(ButtonVariant::Primary, ButtonVariant::Secondary);
    assert_ne!(ButtonVariant::Destructive, ButtonVariant::Ghost);
}

#[test]
fn test_button_variant_clone() {
    let variant = ButtonVariant::Primary;
    let cloned = variant.clone();
    assert_eq!(variant, cloned);
}

#[test]
fn test_button_variant_copy() {
    let variant = ButtonVariant::Secondary;
    let copied: ButtonVariant = variant;
    assert_eq!(variant, copied);
}

#[test]
fn test_button_variant_classes_non_empty() {
    let variants = [
        ButtonVariant::Primary,
        ButtonVariant::Secondary,
        ButtonVariant::Destructive,
        ButtonVariant::Ghost,
        ButtonVariant::Outline,
        ButtonVariant::Link,
    ];

    for variant in variants {
        let class = variant.class();
        assert!(!class.is_empty(), "Variant {:?} should have class", variant);
    }
}

#[test]
fn test_button_variant_classes_unique() {
    let primary = ButtonVariant::Primary.class();
    let secondary = ButtonVariant::Secondary.class();
    let destructive = ButtonVariant::Destructive.class();
    let ghost = ButtonVariant::Ghost.class();
    let outline = ButtonVariant::Outline.class();
    let link = ButtonVariant::Link.class();

    // All classes should be distinct
    assert_ne!(primary, secondary);
    assert_ne!(primary, destructive);
    assert_ne!(primary, ghost);
    assert_ne!(primary, outline);
    assert_ne!(primary, link);
    assert_ne!(secondary, destructive);
    assert_ne!(ghost, outline);
}

#[test]
fn test_button_variant_destructive_has_red() {
    let class = ButtonVariant::Destructive.class();
    assert!(class.contains("red"), "Destructive should contain 'red' color");
}

#[test]
fn test_button_variant_link_has_underline() {
    let class = ButtonVariant::Link.class();
    assert!(class.contains("underline"), "Link variant should have underline styles");
}

#[test]
fn test_button_variant_outline_has_border() {
    let class = ButtonVariant::Outline.class();
    assert!(class.contains("border"), "Outline variant should have border");
}

#[test]
fn test_button_variant_ghost_has_hover() {
    let class = ButtonVariant::Ghost.class();
    assert!(class.contains("hover:"), "Ghost variant should have hover styles");
}

// ========================================================================
// ButtonSize Tests
// ========================================================================

#[test]
fn test_button_size_default() {
    assert_eq!(ButtonSize::default(), ButtonSize::Default);
}

#[test]
fn test_button_size_equality() {
    assert_eq!(ButtonSize::Default, ButtonSize::Default);
    assert_eq!(ButtonSize::Sm, ButtonSize::Sm);
    assert_eq!(ButtonSize::Lg, ButtonSize::Lg);
    assert_eq!(ButtonSize::Icon, ButtonSize::Icon);

    assert_ne!(ButtonSize::Default, ButtonSize::Sm);
    assert_ne!(ButtonSize::Lg, ButtonSize::Icon);
}

#[test]
fn test_button_size_classes_non_empty() {
    let sizes = [
        ButtonSize::Default,
        ButtonSize::Sm,
        ButtonSize::Lg,
        ButtonSize::Icon,
    ];

    for size in sizes {
        let class = size.class();
        assert!(!class.is_empty(), "Size {:?} should have class", size);
    }
}

#[test]
fn test_button_size_classes_unique() {
    let default = ButtonSize::Default.class();
    let sm = ButtonSize::Sm.class();
    let lg = ButtonSize::Lg.class();
    let icon = ButtonSize::Icon.class();

    assert_ne!(default, sm);
    assert_ne!(default, lg);
    assert_ne!(default, icon);
    assert_ne!(sm, lg);
    assert_ne!(lg, icon);
}

#[test]
fn test_button_size_sm_smaller_text() {
    let class = ButtonSize::Sm.class();
    assert!(class.contains("text-xs"), "Sm size should have smaller text");
}

#[test]
fn test_button_size_icon_square() {
    let class = ButtonSize::Icon.class();
    assert!(class.contains("w-9"), "Icon size should be square");
    assert!(class.contains("h-9"), "Icon size should be square");
}

#[test]
fn test_button_size_lg_larger_padding() {
    let class = ButtonSize::Lg.class();
    assert!(class.contains("px-8"), "Lg size should have larger padding");
}

// ========================================================================
// BadgeVariant Tests
// ========================================================================

#[test]
fn test_badge_variant_default() {
    assert_eq!(BadgeVariant::default(), BadgeVariant::Default);
}

#[test]
fn test_badge_variant_equality() {
    assert_eq!(BadgeVariant::Default, BadgeVariant::Default);
    assert_eq!(BadgeVariant::Success, BadgeVariant::Success);
    assert_eq!(BadgeVariant::Warning, BadgeVariant::Warning);
    assert_eq!(BadgeVariant::Danger, BadgeVariant::Danger);
    assert_eq!(BadgeVariant::Info, BadgeVariant::Info);

    assert_ne!(BadgeVariant::Default, BadgeVariant::Success);
    assert_ne!(BadgeVariant::Warning, BadgeVariant::Danger);
}

#[test]
fn test_badge_variant_clone() {
    let variant = BadgeVariant::Success;
    let cloned = variant.clone();
    assert_eq!(variant, cloned);
}

#[test]
fn test_badge_variant_copy() {
    let variant = BadgeVariant::Warning;
    let copied: BadgeVariant = variant;
    assert_eq!(variant, copied);
}

// ========================================================================
// Combined Tests
// ========================================================================

#[test]
fn test_all_button_variants_exist() {
    // Compile-time check that all expected variants exist
    let _primary = ButtonVariant::Primary;
    let _secondary = ButtonVariant::Secondary;
    let _destructive = ButtonVariant::Destructive;
    let _ghost = ButtonVariant::Ghost;
    let _outline = ButtonVariant::Outline;
    let _link = ButtonVariant::Link;
}

#[test]
fn test_all_button_sizes_exist() {
    // Compile-time check that all expected sizes exist
    let _default = ButtonSize::Default;
    let _sm = ButtonSize::Sm;
    let _lg = ButtonSize::Lg;
    let _icon = ButtonSize::Icon;
}

#[test]
fn test_all_badge_variants_exist() {
    // Compile-time check that all expected variants exist
    let _default = BadgeVariant::Default;
    let _success = BadgeVariant::Success;
    let _warning = BadgeVariant::Warning;
    let _danger = BadgeVariant::Danger;
    let _info = BadgeVariant::Info;
}