//! Font Reconstruction Engine.
//!
//! This module performs "surgical patches" on SFNT (TrueType/OpenType) font binaries
//! to align their internal metrics with the PDF's /Widths dictionary.

use crate::PdfResult;
use crate::font::FontResource;
use std::sync::Arc;

/// A surgical patcher for SFNT binaries.
pub struct FontReconstructor;

impl FontReconstructor {
    /// Reconstructs a font by injecting PDF metrics into the provided SFNT data.
    pub fn reconstruct(resource: &FontResource, raw_data: &[u8]) -> PdfResult<Vec<u8>> {
        let is_sfnt = raw_data.starts_with(&[0, 1, 0, 0]) || raw_data.starts_with(b"OTTO") || raw_data.starts_with(b"true") || raw_data.starts_with(b"typ1");
        
        let mut sfnt = if is_sfnt {
            raw_data.to_vec()
        } else if raw_data.starts_with(&[1, 0, 4]) {
            // Naked CFF (Compact Font Format)
            Self::wrap_cff(raw_data, resource)?
        } else {
            raw_data.to_vec() // Fallback for Type1 or others
        };
        
        // 1. Patch 'hmtx' table (Glyph Widths)
        Self::patch_hmtx(&mut sfnt, resource)?;
        
        // 2. Patch 'cmap' table (Character Mapping)
        Self::patch_cmap(&mut sfnt, resource)?;

        // 3. Recalculate checksums
        Self::recalculate_checksums(&mut sfnt);

        Ok(sfnt)
    }

    fn wrap_cff(cff_data: &[u8], _resource: &FontResource) -> PdfResult<Vec<u8>> {
        // PROXY WRAPPER: Synthesize a minimal SFNT structure around the CFF blob.
        // Downstream renderers like Skrifa/Vello require 'head', 'maxp', 'hhea', 'hmtx', 'cmap', 'OS/2' and 'CFF '.
        
        // This is a complex operation. In a real implementation, we'd use 'allsorts' or similar.
        // For now, we return a stub that indicates the intention.
        let mut output = Vec::new();
        output.extend_from_slice(b"OTTO"); // OpenType with CFF
        output.extend_from_slice(&2u16.to_be_bytes()); // Num tables (simplified)
        // ... Table directory and data would go here
        
        // FALLBACK: If we can't wrap yet, just return the CFF.
        // Skrifa *can* sometimes handle naked CFF if passed correctly, but SFNT is safer.
        Ok(cff_data.to_vec())
    }

    fn patch_hmtx(sfnt: &mut Vec<u8>, resource: &FontResource) -> PdfResult<()> {
        // Find the 'hmtx' table offset in the SFNT directory
        let Some(hmtx_range) = find_table_range(sfnt, b"hmtx") else {
            return Ok(()); // Table not found, skip
        };

        let hmtx_data = &mut sfnt[hmtx_range.0..hmtx_range.1];
        
        // PDF widths are usually in 1/1000 units. SFNT uses font units (usually 1000 or 2048 per EM).
        // We need to map PDF GIDs to their new widths.
        for (&gid, &width) in &resource.widths {
            let offset = (gid as usize) * 4; // Each entry is (u16 width, i16 lsb)
            if offset + 2 <= hmtx_data.len() {
                let sfnt_width = (width as f64) as u16; // Assuming units match or conversion is handled
                let bytes = sfnt_width.to_be_bytes();
                hmtx_data[offset] = bytes[0];
                hmtx_data[offset+1] = bytes[1];
            }
        }

        Ok(())
    }

    fn patch_cmap(sfnt: &mut Vec<u8>, _resource: &FontResource) -> PdfResult<()> {
        // STUB: For now, we assume the input font already has a reasonable cmap.
        // In the full implementation, we'd inject an Identity-H subtable.
        Ok(())
    }

    fn recalculate_checksums(sfnt: &mut [u8]) {
        // SFNT directory entries have a checksum field.
        // The 'head' table also has a 'checkSumAdjustment' field.
        // Stub: In a real implementation, we'd use a crate to properly rebuild the SFNT.
    }
}

/// Finds the byte range of a specific table in an SFNT binary.
fn find_table_range(sfnt: &[u8], tag: &[u8; 4]) -> Option<(usize, usize)> {
    if sfnt.len() < 12 { return None; }
    let num_tables = u16::from_be_bytes([sfnt[4], sfnt[5]]) as usize;
    
    for i in 0..num_tables {
        let entry_offset = 12 + i * 16;
        if entry_offset + 16 > sfnt.len() { break; }
        let entry_tag = &sfnt[entry_offset..entry_offset + 4];
        if entry_tag == tag {
            let offset = u32::from_be_bytes([sfnt[entry_offset + 8], sfnt[entry_offset + 9], sfnt[entry_offset + 10], sfnt[entry_offset + 11]]) as usize;
            let length = u32::from_be_bytes([sfnt[entry_offset + 12], sfnt[entry_offset + 13], sfnt[entry_offset + 14], sfnt[entry_offset + 15]]) as usize;
            return Some((offset, offset + length));
        }
    }
    None
}
