# UI Component Design Protocol

> [!IMPORTANT]
> The Ferruginous UI balances "Data Purity" with "Intuitive Operability." Leverage Egui's immediate mode characteristics to always project a Single Source of Truth (SDK state).

---

## 1. SDK State Projection
- **Rule**: Minimize internal UI state and derive visibility/content directly from `ferruginous-sdk` data types.
- **Purpose**: Eliminate inconsistencies between the interface and the underlying PDF document state.
- **Compliance Criterion**: UI components must accept SDK-native types as primary arguments for rendering.

## 2. Premium Design Language
- **Rule**: Utilize an HSL-based harmonious color palette, high-precision typography, and 100-200ms micro-animations.
- **Purpose**: Provide a professional-grade experience that reflects the "Reference Precision" of the toolkit.
- **Compliance Criterion**: Elimination of browser default styles and adherence to the unified Design Token set.

## 3. Layout Stability
- **Rule**: assign unique IDs to all interactive elements and ensure layout resilience against window resizing.
- **Purpose**: Prevent visual breakage and ensure testability through automated UI drivers.
- **Compliance Criterion**: Zero rendering artifacts or crashes during viewport resizing or high-dpi scaling changes.

## 4. Visual Accessibility
- **Rule**: Maintain a minimum contrast ratio of 4.5:1 and provide first-class support for both Light and Dark themes.
- **Purpose**: Ensure that document intelligence is accessible to all users across various lighting conditions.
- **Compliance Criterion**: Passing accessibility audits and seamless theme switching.
