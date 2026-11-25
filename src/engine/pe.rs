// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Analyse statique d'un exécutable Windows (format PE), via goblin. On n'exécute JAMAIS le fichier :
//! on le lit, on liste ses sections, ses fonctions importées, et on localise son code pour le désassembler.

use goblin::pe::PE;

const SCN_MEM_EXECUTE: u32 = 0x2000_0000;
const SCN_MEM_WRITE: u32 = 0x8000_0000;

#[derive(Debug, Clone)]
pub struct SectionInfo {
    pub nom: String,
    pub taille_virtuelle: u32,
    pub taille_raw: u32,
    pub ptr_raw: u32,
    pub virtual_address: u32,
    pub executable: bool,
    pub inscriptible: bool,
}

#[derive(Debug, Clone)]
pub struct InfoPe {
    pub est_64: bool,
    pub entry_rva: u64,
    pub image_base: u64,
    pub sections: Vec<SectionInfo>,
    pub imports: Vec<String>, // noms des fonctions importées
    pub resume: Vec<(String, String)>,
}

/// Parse un PE. Renvoie `Err` si ce n'est pas un exécutable Windows (l'appelant fait alors une analyse limitée).
pub fn analyser(octets: &[u8]) -> Result<InfoPe, String> {
    let pe = PE::parse(octets).map_err(|e| format!("pas un PE Windows : {e}"))?;

    let sections: Vec<SectionInfo> = pe
        .sections
        .iter()
        .map(|s| {
            let nom = s.name().unwrap_or("?").to_string();
            SectionInfo {
                nom,
                taille_virtuelle: s.virtual_size,
                taille_raw: s.size_of_raw_data,
                ptr_raw: s.pointer_to_raw_data,
                virtual_address: s.virtual_address,
                executable: s.characteristics & SCN_MEM_EXECUTE != 0,
                inscriptible: s.characteristics & SCN_MEM_WRITE != 0,
            }
        })
        .collect();

    let imports: Vec<String> = pe.imports.iter().map(|i| i.name.to_string()).collect();
    let libraries: Vec<String> = pe.libraries.iter().map(|l| l.to_string()).collect();

    let machine = pe.header.coff_header.machine;
    let arch = match machine {
        0x8664 => "x86-64 (64 bits)",
        0x014c => "x86 (32 bits)",
        0xAA64 => "ARM64",
        _ => "inconnue",
    };

    let mut resume = Vec::new();
    resume.push(("Architecture".into(), arch.into()));
    resume.push(("Sections".into(), sections.len().to_string()));
    resume.push(("Point d'entrée (RVA)".into(), format!("0x{:X}", pe.entry)));
    resume.push(("Base de l'image".into(), format!("0x{:X}", pe.image_base)));
    resume.push(("Fonctions importées".into(), imports.len().to_string()));
    let apercu_dll = if libraries.len() > 5 {
        format!("{} : {}…", libraries.len(), libraries[..5].join(", "))
    } else {
        format!("{} : {}", libraries.len(), libraries.join(", "))
    };
    resume.push(("Bibliothèques (DLL)".into(), apercu_dll));
    resume.push(("Signature Authenticode".into(), "non vérifiée pour l'instant — et son absence n'est pas une preuve".into()));

    Ok(InfoPe {
        est_64: pe.is_64,
        entry_rva: pe.entry as u64,
        image_base: pe.image_base as u64,
        sections,
        imports,
        resume,
    })
}

/// Localise les octets du code au point d'entrée (mappe la RVA vers un décalage de fichier via les sections).
pub fn code_entree<'a>(octets: &'a [u8], info: &InfoPe) -> Option<&'a [u8]> {
    let rva = info.entry_rva;
    for s in &info.sections {
        let va = s.virtual_address as u64;
        let taille = s.taille_virtuelle.max(s.taille_raw) as u64;
        if rva >= va && rva < va + taille {
            let delta = rva - va;
            let debut = s.ptr_raw as u64 + delta;
            let fin = ((s.ptr_raw as u64) + s.taille_raw as u64).min(octets.len() as u64);
            if debut < fin {
                return Some(&octets[debut as usize..fin as usize]);
            }
        }
    }
    None
}
