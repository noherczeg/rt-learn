/* Linker memory map for the STMicroelectronics STM32C562RE.
 *
 * Source: STM32C562RE datasheet (STM32C5 series, Arm Cortex-M33).
 *   - 512 KB flash memory (main array), base 0x0800_0000
 *   - 128 KB SRAM (contiguous), base 0x2000_0000  (64 KB of which is ECC-backed)
 *
 * cortex-m-rt's link.x consumes these regions. `_stack_start` defaults to the end
 * of RAM; flip-link then relocates the stack, so no manual stack symbol is needed.
 */
MEMORY
{
    FLASH : ORIGIN = 0x08000000, LENGTH = 512K
    RAM   : ORIGIN = 0x20000000, LENGTH = 128K
}
