// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Noam Lellouche — NaAnSentinels (projet original)
//! Désassemblage x86/x64 via iced-x86 — le cœur « reverse engineering » : on lit l'assembleur du fichier
//! AVANT toute exécution. Chaque ligne : adresse, octets, instruction (syntaxe NASM).

use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, NasmFormatter};

/// Désassemble au plus `max` instructions. `bitness` vaut 64 ou 32 ; `rip` est l'adresse de départ affichée.
pub fn desassembler(code: &[u8], bitness: u32, rip: u64, max: usize) -> Vec<String> {
    let mut decoder = Decoder::with_ip(bitness, code, rip, DecoderOptions::NONE);
    let mut formatter = NasmFormatter::new();
    let mut instr = Instruction::default();
    let mut texte = String::new();
    let mut lignes = Vec::new();

    while decoder.can_decode() && lignes.len() < max {
        decoder.decode_out(&mut instr);
        texte.clear();
        formatter.format(&instr, &mut texte);

        // Octets bruts de l'instruction (pour voir le code machine, comme un vrai désassembleur).
        let debut = (instr.ip().wrapping_sub(rip)) as usize;
        let longueur = instr.len();
        let octets = code
            .get(debut..debut + longueur)
            .map(|o| o.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(""))
            .unwrap_or_default();

        lignes.push(format!("{:016X}  {:<20}  {}", instr.ip(), octets, texte));
    }
    lignes
}
