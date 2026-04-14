# Naming Convention Protocol (RFC 0430 Alignment)

This document defines the official naming conventions for the Ferruginous project, adhering to [Rust RFC 0430](https://github.com/rust-lang/rfcs/blob/master/text/0430-finalizing-naming-conventions.md).

> [!IMPORTANT]
> **Priority**: If there is a conflict between the terminology in the PDF specification (ISO 32000-2) and Rust conventions, **Rust's common naming conventions (Naming Context) shall prevail** as a general rule.

---

## 1. Casing Strategy

- **Rule**: Use `UpperCamelCase` for types and traits, and `snake_case` for functions and variables.
- **Purpose**: Align with Rust compilation conventions (RFC 0430) to eliminate warnings and maximize readability.
- **Criterion**: Zero naming warnings from Clippy.

## 2. Getters & Conversions

- **Rule**: Avoid using `get_` for direct retrieval. Use `as_` (reference), `to_` (creation), and `into_` (consumption) to differentiate conversions.
- **Purpose**: Make Rust's unique ownership semantics intuitively understandable from method names.
- **Criterion**: Exclusion of the `get_` prefix in the SDK's public API signatures.

## 3. PDF Domain Mapping

- **Rule**: Retain PDF specification terminology (e.g., MediaBox) but adhere to Rust casing conventions (e.g., `media_box`). Treat abbreviations as normal words (e.g., `PdfError`).
- **Purpose**: Seamlessly fuse domain-specific knowledge with language conventions.
- **Criterion**: Key names from the Arlington Model and Rust method names must be mechanically convertible.

---

## 4. Naming Correspondence Table

| Item | Style | Example |
| :--- | :--- | :--- |
| **Types, Traits** | `UpperCamelCase` | `PdfResolver`, `Object` |
| **Enum Variants** | `UpperCamelCase` | `Dictionary`, `String` |
| **Functions, Methods** | `snake_case` | `resolve()`, `media_box()` |
| **Variables, Fields** | `snake_case` | `local_name`, `page_dict` |
| **Constants** | `SCREAMING_SNAKE` | `MAX_STREAM_SIZE` |
