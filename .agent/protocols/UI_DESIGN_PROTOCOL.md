# UI Component Design Protocol

> [!IMPORTANT]
> The Ferruginous UI balances "Data Purity" with "Intuitive Operability." Leverage Egui's immediate mode characteristics to always visualize a Single Source of Truth (SDK state).

## 1. Shared SSoT (Shared Single Source of Truth)

- **Rule**: Minimize the UI's own mutable state and directly project the state of `ferruginous-sdk`.
- **Purpose**: Physically eliminate inconsistencies between the UI and data, maintaining a single truth.
- **Criterion**: UI components must accept SDK data types directly as arguments.

## 2. Premium Aesthetics

- **Rule**: Adopt an HSL-based color palette, modern typography, and 100-200ms micro-animations.
- **Purpose**: Provide a high-quality user experience that not only offers functionality but also satisfies the desire for ownership.
- **Criterion**: Browser default styles must be completely eliminated, and consistent design tokens must be applied.

## 3. Fluidity & Robustness

- **Rule**: Assign a unique ID to every interactive element and ensure they follow window resizing and invalid input without crashing.
- **Purpose**: Improve testability and ensure stability across all usage environments.
- **Criterion**: No rendering breakdown during resizing, and all elements must hold a `Universal ID`.

---

## 4. Audit Checklist

1.  **SSoT Projection**: Is there any divergence from the SDK state?
2.  **Performance**: Is 60 FPS being maintained?
3.  **Visual Consistency**: Are margins, colors, and typography unified?
4.  **Accessibility**: Is a contrast ratio of 4.5:1 or higher maintained, and is dark mode supported?
