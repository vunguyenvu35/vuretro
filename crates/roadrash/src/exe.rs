//! Tables that are kept in `ROADRASH.EXE` rather than in the data files.
//!
//! The disc's executable is 541,184 bytes long. Its data section is loaded at address
//! `0x0046_3000` from file offset `0x6_1600`; the colour tables of the riders are 15 tables of
//! 256 bytes at address `0x0046_E158`. A rider sprite is drawn with a template palette, and
//! each table says which index of the real palette stands for each template index, one table
//! for every colour a rider can have. The first two indices (0, see-through, and 1) keep
//! their numbers in every table.

const SIZE: usize = 541_184;
const DATA_ADDRESS: usize = 0x0046_3000;
const DATA_OFFSET: usize = 0x6_1600;
const TABLES_ADDRESS: usize = 0x0046_E158;
/// How many colours a rider can have.
pub const RIDER_COLOURS: usize = 15;

/// One table: the real palette index for each index of the sprite.
pub type Remap = [u8; 256];

/// The rider colour tables of the disc's executable.
pub fn rider_remaps(exe: &[u8]) -> Result<Vec<Remap>, String> {
    if exe.len() != SIZE {
        return Err(format!(
            "ROADRASH.EXE is {} bytes; the tables are known for the disc's {SIZE}-byte one",
            exe.len()
        ));
    }
    let start = DATA_OFFSET + (TABLES_ADDRESS - DATA_ADDRESS);
    let tables: Vec<Remap> = (0..RIDER_COLOURS)
        .map(|k| {
            let mut t = [0u8; 256];
            t.copy_from_slice(&exe[start + k * 256..start + (k + 1) * 256]);
            t
        })
        .collect();
    if tables.iter().any(|t| t[0] != 0 || t[1] != 1) {
        return Err("the tables are not where they were in the disc's executable".into());
    }
    Ok(tables)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exe_with_tables() -> Vec<u8> {
        let mut exe = vec![0u8; SIZE];
        let start = DATA_OFFSET + (TABLES_ADDRESS - DATA_ADDRESS);
        for k in 0..RIDER_COLOURS {
            for i in 0..256 {
                exe[start + k * 256 + i] = if i < 2 {
                    i as u8
                } else {
                    (i as u8).wrapping_add(k as u8)
                };
            }
        }
        exe
    }

    #[test]
    fn the_tables_are_taken_from_where_the_executable_keeps_them() {
        let tables = rider_remaps(&exe_with_tables()).unwrap();
        assert_eq!(tables.len(), 15);
        assert_eq!(tables[3][10], 13);
        assert_eq!(tables[0][1], 1);
    }

    #[test]
    fn another_executable_is_refused() {
        assert!(rider_remaps(&[0; 100]).is_err());
        assert!(
            rider_remaps(&vec![9u8; SIZE]).is_err(),
            "right size, tables not there"
        );
    }
}
