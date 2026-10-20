use fountain_codes::*;
pub fn encoder() -> Result<RepairableEncoder, FountainError> {
    // Experimental parameters from the independent v1 fixture, no probability guarantee.
    let config = RepairableConfig::try_new(
        Profile::Gf65536,
        4,
        5,
        0x0123456789abcdef,
        ResourceLimits::default(),
    )?;
    RepairableEncoder::try_new(
        &[
            0x80, 0x01, 0xff, 0x00, 0x34, 0x12, 0x00, 0x80, 0xaa, 0x55, 0xcc,
        ],
        [0; 16],
        config,
    )
}
