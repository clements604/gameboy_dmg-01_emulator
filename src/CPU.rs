
const ZERO_FLAG_BYTE_POSITION: u8 = 7;
const SUBTRACT_FLAG_BYTE_POSITION: u8 = 6;
const HALF_CARRY_FLAG_BYTE_POSITION: u8 = 5;
const CARRY_FLAG_BYTE_POSITION: u8 = 4;

struct CPU {

}

struct Registers {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: FlagsRegister,
    h: u8,
    l: u8,
}

impl Registers {

    fn get_bc(&self) -> u16 {
        (self.b as u16) << 8 | self.c as u16;
    }

    fn set_bc(&mut self, value: u16) {
        self.b = ((value & 0xFF00) >> 8) as u8;
        self.c = (value & 0xFF) as u8;
    }

}

struct FlagsRegister {
    zero: bool,
    subtract: bool,
    half_carry: bool,
    carry: bool,
}

impl std::convert::From<FlagsRegister> for u8 {
    fn from(flag: FlagsRegister) -> u8 {
        (if flag.zero       {1} else {0}) << ZERO_FLAG_BYTE_POSITION |
        (if flag.subtract   {1} else {0}) << SUBTRACT_FLAG_BYTE_POSITION |
        (if flag.half_carry {1} else {0}) << HALF_CARRY_FLAG_BYTE_POSITION |
        (if flag.carry      {1} else {0}) << CARRY_FLAG_BYTE_POSITION
    }
}

impl std::convert::From<u8> for FlagsRegister {
    fn from(byte: u8) -> Self {
        let zero = ((byte >> ZERO_FLAG_BYTE_POSITION) & 0b1) != 0;
        let subtract = ((byte >> SUBTRACT_FLAG_BYTE_POSITION) &0b1) != 0;
        let half_carry = ((byte >> HALF_CARRY_FLAG_BYTE_POSITION) & 0b1) != 0;
        let carry = ((byte >> CARRY_FLAG_BYTE_POSITION) & 0b1) != 0;

        FLAGS_REGISTER {
            zero, // Set to true if the result of the operation is equal to 0
            subtract, // Set to true if the operation was a subtraction.
            /*
            set to true if there is an overflow from the lower nibble (a.k.a the lower four bits) to the upper nibble (a.k.a the upper four bits). Let's take a look at some examples of what this means. In the following diagram, we have the byte 143 in binary (0b1000_1111). We then add 0b1 to the number. Notice how the 1 from the lower nibble is carried to the upper nibble. You should already be familiar with carries from elemetry arithmetic. Whenever there's not enough room for a number in a particular digit's place, we carry over to the next digits place.

                lower nibble            lower nibble
                    ┌--┐                    ┌--┐
                1000 1111  +   1   ==   1001 0000
                └--┘                    └--┘
                upper nibble            upper nibble
            
            If this happens when adding our values, we set the half_carry flag to true. We can test for this by masking out the upper nibble of both the A register and the value we're adding and testing if this value is greater than 0xF.
             */
            half_carry,
            carry, // Set to true if the operation resulted in an overflow.
        }
    }
}

enum Instruction {
    ADD(ArithmeticTarget),
}

enum ArithmeticTarget {
    A,
    B,
    C,
    D,
    E,
    H,
    L
}

impl CPU {

    fn execute(&mut self, instruction: Instruction) {
        match instruction {
          Instruction::ADD(target) => {
            match target {
                ArithmeticTarget::C => {
                    let value = self.registers.c;
                    let new_value = self.add(value);
                    self.registers.a = new_value;
                }
                _ => { /* TODO Implement other targets */ }
            }
          }
          _ => {
              //TODO: Implement other instructions
          }
        }
    }

    fn add(&mut self, value: u8) -> u8 {
        let (new_value, did_overflow) = self.registers.a.overflowing_add(value);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = did_overflow;
        // Half Carry is set if adding the lower nibbles of the value and register A
        // together result in a value bigger than 0xF. If the result is larger than 0xF
        // than the addition caused a carry from the lower nibble to the upper nibble.
        self.registers.f.half_carry = (self.registers.a & 0xF) + (value & 0xF) > 0xF;
        new_value
    }

    /*
    * Adds the value to the HL register.
    * HL register is a 16-bit value derrived from the H and L registers.
    * TODO TEST
    */
    fn addhl(&mut self, value: u16) -> u16 {
        let current_value: u16 = (u16::from(self.registers.h) << 8) | u16::from(self.registers.l);
        let (new_value, did_overflow) = current_value.carrying_add(value);
        self.registers.h = ((result & 0xFF00) >> 8) as u8;
        self.registers.l = (result & 0xFF) as u8;
        self.registers.f.zero - new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = (self.registers.h & 0xF) + (value & 0xF) > 0xF;
    }

    /*
    * Add with carry.
    * Same as add, however the value of the carry flag is added to the result.
     */
    fn adc(&mut self, value: u8) -> u8 {
        let (new_value, did_overflow) = self.registers.a.carrying_add(value);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = (self.registers.a & 0xF) + (value & 0xF) > 0xF;
        new_value
    }

    /*
    * Subtract
    * Subtract the value stored in a specific register with the value in the A register
    * TODO TEST
    */
    fn sub(&mut self, register: u8) -> u8 {
        let (new_value, did_overflow) = register.overflowing_sub(value);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = true;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = (self.registers.a & 0xF) < (value & 0xF);
        new_value
    }

    /*
    * Subtract with carry
    * Just like ADD except that the value of the carry flag is also subtracted from the number
    * TODO TEST
    */
    fn sbc(&mut self, value: u8) -> u8 {
        let (new_value, did_overflow) = self.registers.a.borrowing_sub(value);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = true;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = (self.registers.a & 0xF) - (value & 0xF);
        new_value
    }

    /*
    * Logical AND
    * The value of the A register is bitwise ANDed with the value of the specified register.
    * TODO TEST
    */
    fn and(&mut self, value: u8) -> u8 {
        let new_value: u8 = value & self.registers.a;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = false;
        self.registers.f.half_carry = false;
        new_value
    }

    /*
    * Logical OR
    * The value of the A register is bitwise ORed with the value of the specified register.
    * TODO TEST
    */
    fn or(&mut self, value: u8) -> u8 {
        let new_value: u8 = value | self.registers.a;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = false;
        self.registers.f.half_carry = false;
        new_value
    }

    /*
    * Logical XOR
    * The value of the A register is bitwise XORed with the value of the specified register.
    * TODO TEST
    */
    fn xor(&mut self, value: u8) -> u8 {
        let new_value: u8 = value ^ self.registers.a;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = false;
        self.registers.f.half_carry = false;
        new_value
    }

    /*
    * Compare
    * Just like SUB except the result of the subtraction is not stored back into A.
    * TODO TEST
    */
    fn cp(self, value: u8) {
        let (new_value, did_overflow) = self.registers.a.overflowing_sub(value);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = true;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = (self.registers.a & 0xF) - (value & 0xF);
    }

    /*
    * Increment
    * Increment the value in a specific register by 1.
    * TODO TEST
    */
    fn inc(&mut self, register: &mut u8) -> u8 {
        let (new_value, did_overflow) = register.overflowing_add(1);
        //*register = new_value;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = (*register & 0xF) + (1 & 0xF) > 0xF;
        new_value
    }

    /*
    * Decrement
    * Decrement the value in a specific register by 1.
    * TODO TEST
    */
    fn dec(&mut self, register: &mut u8) -> u8 {
        let (new_value, did_overflow) = register.overflowing_sub(1);
        //*register = new_value;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = true;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = (*register & 0xF) - (1 & 0xF) > 0xF;
        new_value
    }

}