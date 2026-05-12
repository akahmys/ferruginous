//! PDF Content Stream Serializer (Desublimation).
//!
//! This module converts high-level `Command` IR back into physical PDF operators.

use crate::object::sublimation::{Command, IrObject, TextArrayItem};
use kurbo::{Affine, Point};

/// Serializes a sequence of commands into a valid PDF content stream.
pub fn serialize_commands(cmds: &[Command]) -> Vec<u8> {
    let mut buffer = Vec::new();
    for cmd in cmds {
        serialize_command(cmd, &mut buffer);
    }
    buffer
}

fn serialize_command(cmd: &Command, buf: &mut Vec<u8>) {
    match cmd {
        Command::PushState => buf.extend_from_slice(b"q\n"),
        Command::PopState => buf.extend_from_slice(b"Q\n"),
        Command::Transform(affine) => {
            write_affine(affine, buf);
            buf.extend_from_slice(b" cm\n");
        }
        Command::MoveTo(p) => {
            write_point(p, buf);
            buf.extend_from_slice(b" m\n");
        }
        Command::LineTo(p) => {
            write_point(p, buf);
            buf.extend_from_slice(b" l\n");
        }
        Command::CurveTo(p1, p2, p3) => {
            write_point(p1, buf);
            buf.push(b' ');
            write_point(p2, buf);
            buf.push(b' ');
            write_point(p3, buf);
            buf.extend_from_slice(b" c\n");
        }
        Command::ClosePath => buf.extend_from_slice(b"h\n"),
        Command::Rect(rect) => {
            buf.extend_from_slice(
                format!("{} {} {} {} re\n", rect.x0, rect.y0, rect.width(), rect.height())
                    .as_bytes(),
            );
        }
        Command::Fill(winding) => match winding {
            crate::graphics::WindingRule::NonZero => buf.extend_from_slice(b"f\n"),
            crate::graphics::WindingRule::EvenOdd => buf.extend_from_slice(b"f*\n"),
        },
        Command::Stroke(_) => buf.extend_from_slice(b"S\n"),
        Command::FillStroke(winding, _) => match winding {
            crate::graphics::WindingRule::NonZero => buf.extend_from_slice(b"B\n"),
            crate::graphics::WindingRule::EvenOdd => buf.extend_from_slice(b"B*\n"),
        },
        Command::Clip(winding) => match winding {
            crate::graphics::WindingRule::NonZero => buf.extend_from_slice(b"W n\n"),
            crate::graphics::WindingRule::EvenOdd => buf.extend_from_slice(b"W* n\n"),
        },
        Command::BeginText => buf.extend_from_slice(b"BT\n"),
        Command::EndText => buf.extend_from_slice(b"ET\n"),
        Command::SetFont { font, size } => {
            buf.extend_from_slice(format!("/{} {} Tf\n", font, size).as_bytes());
        }
        Command::SetFillColor(color) => {
            let rgb = color.to_rgb();
            if let crate::graphics::Color::Rgb(r, g, b) = rgb {
                buf.extend_from_slice(format!("{:.4} {:.4} {:.4} rg\n", r, g, b).as_bytes());
            }
        }
        Command::SetStrokeColor(color) => {
            let rgb = color.to_rgb();
            if let crate::graphics::Color::Rgb(r, g, b) = rgb {
                buf.extend_from_slice(format!("{:.4} {:.4} {:.4} RG\n", r, g, b).as_bytes());
            }
        }
        Command::ShowText(bytes) => {
            buf.push(b'<');
            for &b in bytes {
                buf.extend_from_slice(format!("{:02x}", b).as_bytes());
            }
            buf.extend_from_slice(b"> Tj\n");
        }
        Command::ShowTextArray(items) => {
            buf.push(b'[');
            for item in items {
                match item {
                    TextArrayItem::Text(b) => {
                        buf.push(b'<');
                        for &byte in b {
                            buf.extend_from_slice(format!("{:02x}", byte).as_bytes());
                        }
                        buf.push(b'>');
                    }
                    TextArrayItem::Offset(o) => {
                        buf.extend_from_slice(format!(" {}", o).as_bytes());
                    }
                }
            }
            buf.extend_from_slice(b"] TJ\n");
        }
        Command::MoveText(p) => {
            buf.extend_from_slice(format!("{} {} Td\n", p.x, p.y).as_bytes());
        }
        Command::SetTextMatrix(affine) => {
            write_affine(affine, buf);
            buf.extend_from_slice(b" Tm\n");
        }
        Command::SetCharSpacing(s) => buf.extend_from_slice(format!("{} Tc\n", s).as_bytes()),
        Command::SetWordSpacing(s) => buf.extend_from_slice(format!("{} Tw\n", s).as_bytes()),
        Command::SetHorizontalScaling(s) => buf.extend_from_slice(format!("{} Tz\n", s).as_bytes()),
        Command::SetTextRenderMode(m) => {
            buf.extend_from_slice(format!("{} Tr\n", *m as i32).as_bytes())
        }
        Command::SetTextRise(s) => buf.extend_from_slice(format!("{} Ts\n", s).as_bytes()),
        Command::SetTextLeading(s) => buf.extend_from_slice(format!("{} TL\n", s).as_bytes()),
        Command::MoveToNextLine => buf.extend_from_slice(b"T*\n"),
        Command::DrawXObject(name) => buf.extend_from_slice(format!("/{} Do\n", name).as_bytes()),
        Command::RawOperator { name, operands } => {
            for op in operands {
                write_ir_object(op, buf);
                buf.push(b' ');
            }
            buf.extend_from_slice(name.as_bytes());
            buf.push(b'\n');
        }
        _ => {} // Other commands like SetWritingMode are internal and don't map to PDF operators
    }
}

fn write_point(p: &Point, buf: &mut Vec<u8>) {
    buf.extend_from_slice(format!("{} {}", p.x, p.y).as_bytes());
}

fn write_affine(a: &Affine, buf: &mut Vec<u8>) {
    let c = a.as_coeffs();
    buf.extend_from_slice(
        format!("{} {} {} {} {} {}", c[0], c[1], c[2], c[3], c[4], c[5]).as_bytes(),
    );
}

fn write_ir_object(obj: &IrObject, buf: &mut Vec<u8>) {
    match obj {
        IrObject::Boolean(b) => buf.extend_from_slice(if *b { b"true" } else { b"false" }),
        IrObject::Integer(i) => buf.extend_from_slice(i.to_string().as_bytes()),
        IrObject::Real(f) => buf.extend_from_slice(format!("{:.4}", f).as_bytes()),
        IrObject::String(b) => {
            buf.push(b'(');
            buf.extend_from_slice(&escape_pdf_string(b));
            buf.push(b')');
        }
        IrObject::Hex(b) => {
            buf.push(b'<');
            for &byte in b {
                buf.extend_from_slice(format!("{:02x}", byte).as_bytes());
            }
            buf.push(b'>');
        }
        IrObject::Name(n) => buf.extend_from_slice(format!("/{}", n).as_bytes()),
        IrObject::Null => buf.extend_from_slice(b"null"),
        IrObject::Array(arr) => {
            buf.push(b'[');
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    buf.push(b' ');
                }
                write_ir_object(item, buf);
            }
            buf.push(b']');
        }
        IrObject::Dictionary(dict) => {
            buf.extend_from_slice(b"<< ");
            for (key, val) in dict {
                buf.extend_from_slice(format!("/{} ", key).as_bytes());
                write_ir_object(val, buf);
                buf.push(b' ');
            }
            buf.extend_from_slice(b">>");
        }
    }
}

fn escape_pdf_string(data: &[u8]) -> Vec<u8> {
    let mut escaped = Vec::with_capacity(data.len());
    for &b in data {
        match b {
            b'(' => escaped.extend_from_slice(b"\\("),
            b')' => escaped.extend_from_slice(b"\\)"),
            b'\\' => escaped.extend_from_slice(b"\\\\"),
            _ => escaped.push(b),
        }
    }
    escaped
}

/// Serializes an image back into a compressed PDF stream.
pub fn serialize_image(
    _width: u32,
    _height: u32,
    _format: crate::graphics::PixelFormat,
    data: &[u8],
) -> (Vec<u8>, Vec<String>) {
    // For now, use FlateDecode (lossless) as the default.
    // In a full implementation, we would check the format and potentially use DCTDecode for JPEG.
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    use std::io::Write;
    encoder.write_all(data).unwrap();
    let compressed = encoder.finish().unwrap();

    (compressed, vec!["FlateDecode".to_string()])
}
