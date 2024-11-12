// Basic Configuration and Command Registers
pub const COMMAND_REG: u8 = 0x01 << 1;
pub const COM_IEN_REG: u8 = 0x02 << 1;         // Communication Interrupt Enable Register
pub const DIV_IEN_REG: u8 = 0x03 << 1;         // DivIrq interrupt Enable Register
pub const COMM_IRQ_REG: u8 = 0x04 << 1;        // Interrupt request bits
pub const DIV_IRQ_REG: u8 = 0x05 << 1;         // Set bits to signal internal events
pub const ERROR_REG: u8 = 0x06 << 1;           // Error bits showing the error status of the last command
pub const STATUS1_REG: u8 = 0x07 << 1;         // Communication status bits
pub const STATUS2_REG: u8 = 0x08 << 1;         // Receiver and transmitter status bits
pub const FIFO_DATA_REG: u8 = 0x09 << 1;       // FIFO data input/output
pub const FIFO_LEVEL_REG: u8 = 0x0A << 1;      // Number of bytes in the FIFO buffer
pub const WATER_LEVEL_REG: u8 = 0x0B << 1;     // Level for FIFO underflow and overflow warning
pub const CONTROL_REG: u8 = 0x0C << 1;         // Miscellaneous control bits
pub const BIT_FRAMING_REG: u8 = 0x0D << 1;     // Adjustments for bit-oriented frames
pub const COLL_REG: u8 = 0x0E << 1;            // Collision detection

// Timer and Timeout Configuration
pub const MODE_REG: u8 = 0x11 << 1;            // Defines general modes for transmitting and receiving
pub const T_MODE_REG: u8 = 0x2A << 1;          // TModeReg - Timer settings
pub const T_PRESCALER_REG: u8 = 0x2B << 1;     // TPrescalerReg - Timer prescaler value
pub const T_RELOAD_REG_H: u8 = 0x2C << 1;      // TReloadReg (High) - 16-bit timer reload value (high byte)
pub const T_RELOAD_REG_L: u8 = 0x2D << 1;      // TReloadReg (Low) - 16-bit timer reload value (low byte)

// RF Configuration
pub const TX_MODE_REG: u8 = 0x02 << 1;         // Defines transmission data rate and framing
pub const RX_MODE_REG: u8 = 0x03 << 1;         // Defines reception data rate and framing
pub const TX_CONTROL_REG: u8 = 0x14 << 1;      // Controls the logical behavior of the antenna driver pins TX1 and TX2
pub const TX_ASK_REG: u8 = 0x15 << 1;          // Controls the setting of the transmission modulation
pub const MODE_WIDTH_REG: u8 = 0x24 << 1;      // Modulation width setting (for ASK modulation)
pub const RF_CFG_REG: u8 = 0x26 << 1;          // Configures the receiver gain
pub const GS_N_REG: u8 = 0x27 << 1;            // Conductance of the antenna driver pins
pub const CW_GS_P_REG: u8 = 0x28 << 1;         // Conductance for the modulation signal output
pub const MOD_GS_P_REG: u8 = 0x29 << 1;        // Conductance for the modulation signal output during modulated signal

// CRC and Test Registers
pub const CRC_RESULT_REG_H: u8 = 0x21 << 1;    // CRC calculation result, MSB
pub const CRC_RESULT_REG_L: u8 = 0x22 << 1;    // CRC calculation result, LSB
pub const VERSION_REG: u8 = 0x37 << 1;         // Shows the software version
pub const TEST_SEL1_REG: u8 = 0x31 << 1;       // General test signal configuration
pub const TEST_SEL2_REG: u8 = 0x32 << 1;       // General test signal configuration and PRBS control
pub const TEST_PIN_EN_REG: u8 = 0x33 << 1;     // Enables certain pins to output test signals
pub const TEST_BUS_REG: u8 = 0x34 << 1;        // Controls the pins output driver (for testing)
pub const AUTO_TEST_REG: u8 = 0x36 << 1;       // Controls the self-test
pub const TEST_ADC_REG: u8 = 0x39 << 1;        // Shows the value of ADC I and Q channels
