# Rendering Conventions

Design and implementation conventions for the Ferruginous rendering engine (Core, Render, SDK).

## 1. Text Metrics and Scaling
- **Decoupling Principle**: Generation of the Glyph Path and calculation of Layout (Advance/Metrics) must clearly separate scales.
    - **Path Scale**: `size / units_per_em` (using Font-specific UnitsPerEm).
    - **Metrics Scale**: `size / 1000.0` (using PDF standard 1000-unit system).
- **Rounding**: Manage precision strictly to prevent the accumulation of floating-point errors in layout calculations.

## 2. Coordinate Systems
- **Internal Sovereignty**: All internal logic (Interpreter, FontResource) must consistently use the **Positive Y = UP** coordinate system according to the PDF specification.
- **Conversion Boundary**: Invert the coordinate system (Positive Y = DOWN) ONLY in the layer immediately before sending data to the rendering device (e.g., Vello). Do not flip signs in intermediate pipeline layers.

## 3. Font Resource Inheritance
- **Propagation Obligation**: Inheriting metadata (WMode, Encoding, ToUnicode) from a Type0 font to its CIDFont descendant is mandatory.
- **Resource Loading**: Even when loading a CIDFont in isolation, initialization must always consider the parent context.

## 4. CMap and Encoding Hygiene
- **Isolation**: Each `FontResource` must have its own independent mapping table. "Rescue" logic (using common CMaps) is permitted only for clearly identified CJK fonts and must not have side effects (cache pollution).
- **Strict Parsing**: CMap parsing must accurately handle both literal strings (UTF-16BE) and hex notations.

## 5. Diagnostic Traceability
- **Instrumentation**: Implement trace logs for scaling, matrix transformations, and advancement calculations that can be enabled via environment variables. Do not leave ad-hoc `eprintln!` calls in the code.
