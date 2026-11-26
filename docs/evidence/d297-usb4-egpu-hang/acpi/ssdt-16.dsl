/*
 * Intel ACPI Component Architecture
 * AML/ASL+ Disassembler version 20251212 (64-bit version)
 * Copyright (c) 2000 - 2025 Intel Corporation
 * 
 * Disassembling to symbolic ASL+ operators
 *
 * Disassembly of ssdt-16.aml
 *
 * Original Table Header:
 *     Signature        "SSDT"
 *     Length           0x000018C6 (6342)
 *     Revision         0x02
 *     Checksum         0x0B
 *     OEM ID           "LENOVO"
 *     OEM Table ID     "SocGpe "
 *     OEM Revision     0x00003000 (12288)
 *     Compiler ID      "INTL"
 *     Compiler Version 0x20210930 (539035952)
 */
DefinitionBlock ("", "SSDT", 2, "LENOVO", "SocGpe ", 0x00003000)
{
    External (_SB_.PC00, DeviceObj)
    External (_SB_.PC00.GFX0, DeviceObj)
    External (_SB_.PC00.IMNG, MethodObj)    // 1 Arguments
    External (_SB_.PC00.MC__, DeviceObj)
    External (_SB_.PC00.MC__.D1F0, FieldUnitObj)
    External (_SB_.PC00.MC__.D1F1, FieldUnitObj)
    External (_SB_.PC00.MC__.D6F0, FieldUnitObj)
    External (_SB_.PC00.RP01, DeviceObj)
    External (_SB_.PC00.RP01.HPME, MethodObj)    // 0 Arguments
    External (_SB_.PC00.RP01.HPSX, FieldUnitObj)
    External (_SB_.PC00.RP01.L0SE, FieldUnitObj)
    External (_SB_.PC00.RP01.PDCX, FieldUnitObj)
    External (_SB_.PC00.RP01.PDSX, FieldUnitObj)
    External (_SB_.PC00.RP01.VDID, FieldUnitObj)
    External (_SB_.PC00.RP02, DeviceObj)
    External (_SB_.PC00.RP02.HPME, MethodObj)    // 0 Arguments
    External (_SB_.PC00.RP02.HPSX, FieldUnitObj)
    External (_SB_.PC00.RP02.L0SE, FieldUnitObj)
    External (_SB_.PC00.RP02.PDCX, FieldUnitObj)
    External (_SB_.PC00.RP02.PDSX, FieldUnitObj)
    External (_SB_.PC00.RP02.VDID, FieldUnitObj)
    External (_SB_.PC00.RP03, DeviceObj)
    External (_SB_.PC00.RP03.HPME, MethodObj)    // 0 Arguments
    External (_SB_.PC00.RP03.HPSX, FieldUnitObj)
    External (_SB_.PC00.RP03.L0SE, FieldUnitObj)
    External (_SB_.PC00.RP03.PDCX, FieldUnitObj)
    External (_SB_.PC00.RP03.PDSX, FieldUnitObj)
    External (_SB_.PC00.RP03.VDID, FieldUnitObj)
    External (_SB_.PC00.RP04, DeviceObj)
    External (_SB_.PC00.RP04.HPME, MethodObj)    // 0 Arguments
    External (_SB_.PC00.RP04.HPSX, FieldUnitObj)
    External (_SB_.PC00.RP04.L0SE, FieldUnitObj)
    External (_SB_.PC00.RP04.PDCX, FieldUnitObj)
    External (_SB_.PC00.RP04.PDSX, FieldUnitObj)
    External (_SB_.PC00.RP04.VDID, FieldUnitObj)
    External (_SB_.PC00.RP05, DeviceObj)
    External (_SB_.PC00.RP05.HPME, MethodObj)    // 0 Arguments
    External (_SB_.PC00.RP05.HPSX, FieldUnitObj)
    External (_SB_.PC00.RP05.L0SE, FieldUnitObj)
    External (_SB_.PC00.RP05.PDCX, FieldUnitObj)
    External (_SB_.PC00.RP05.PDSX, FieldUnitObj)
    External (_SB_.PC00.RP05.VDID, FieldUnitObj)
    External (_SB_.PC00.RP06, DeviceObj)
    External (_SB_.PC00.RP06.HPME, MethodObj)    // 0 Arguments
    External (_SB_.PC00.RP06.HPSX, FieldUnitObj)
    External (_SB_.PC00.RP06.L0SE, FieldUnitObj)
    External (_SB_.PC00.RP06.PDCX, FieldUnitObj)
    External (_SB_.PC00.RP06.PDSX, FieldUnitObj)
    External (_SB_.PC00.RP06.VDID, FieldUnitObj)
    External (_SB_.PC00.RP07, DeviceObj)
    External (_SB_.PC00.RP07.HPME, MethodObj)    // 0 Arguments
    External (_SB_.PC00.RP07.HPSX, FieldUnitObj)
    External (_SB_.PC00.RP07.L0SE, FieldUnitObj)
    External (_SB_.PC00.RP07.PDCX, FieldUnitObj)
    External (_SB_.PC00.RP07.PDSX, FieldUnitObj)
    External (_SB_.PC00.RP07.VDID, FieldUnitObj)
    External (_SB_.PC00.RP08, DeviceObj)
    External (_SB_.PC00.RP08.HPME, MethodObj)    // 0 Arguments
    External (_SB_.PC00.RP08.HPSX, FieldUnitObj)
    External (_SB_.PC00.RP08.L0SE, FieldUnitObj)
    External (_SB_.PC00.RP08.PDCX, FieldUnitObj)
    External (_SB_.PC00.RP08.PDSX, FieldUnitObj)
    External (_SB_.PC00.RP08.VDID, FieldUnitObj)
    External (_SB_.PC00.RP09, DeviceObj)
    External (_SB_.PC00.RP09.HPME, MethodObj)    // 0 Arguments
    External (_SB_.PC00.RP09.HPSX, FieldUnitObj)
    External (_SB_.PC00.RP09.L0SE, FieldUnitObj)
    External (_SB_.PC00.RP09.PDCX, FieldUnitObj)
    External (_SB_.PC00.RP09.PDSX, FieldUnitObj)
    External (_SB_.PC00.RP09.VDID, FieldUnitObj)
    External (_SB_.PC00.RP10, DeviceObj)
    External (_SB_.PC00.RP10.HPME, MethodObj)    // 0 Arguments
    External (_SB_.PC00.RP11, DeviceObj)
    External (_SB_.PC00.RP11.HPME, MethodObj)    // 0 Arguments
    External (_SB_.PC00.RP12, DeviceObj)
    External (_SB_.PC00.RP12.HPME, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TDM0, DeviceObj)
    External (_SB_.PC00.TDM1, DeviceObj)
    External (_SB_.PC00.TRP0, DeviceObj)
    External (_SB_.PC00.TRP0.CRPM, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TRP0.HPEV, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TRP0.HPME, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TRP0.LTRC, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TRP1, DeviceObj)
    External (_SB_.PC00.TRP1.CRPM, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TRP1.HPEV, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TRP1.HPME, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TRP1.LTRC, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TRP2, DeviceObj)
    External (_SB_.PC00.TRP2.CRPM, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TRP2.HPEV, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TRP2.HPME, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TRP2.LTRC, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TRP3, DeviceObj)
    External (_SB_.PC00.TRP3.CRPM, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TRP3.HPEV, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TRP3.HPME, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TRP3.LTRC, MethodObj)    // 0 Arguments
    External (_SB_.PC00.TXHC, DeviceObj)
    External (_SB_.PC01, DeviceObj)
    External (ADBG, MethodObj)    // 1 Arguments
    External (GP1E, IntObj)
    External (PCHS, FieldUnitObj)
    External (PCHX, IntObj)
    External (PGRT, FieldUnitObj)

    Scope (\_GPE)
    {
        Method (SL61, 0, NotSerialized)
        {
            If (CondRefOf (GP1E))
            {
                If ((GP1E == Zero))
                {
                    If (CondRefOf (\_SB.PC00.TXHC))
                    {
                        Local0 = Zero
                        Local1 = \_SB.PC00.TRP0.CRPM ()
                        If ((Local1 != 0xFF))
                        {
                            Local0 |= Local1
                        }

                        Local1 = \_SB.PC00.TRP1.CRPM ()
                        If ((Local1 != 0xFF))
                        {
                            Local0 |= Local1
                        }

                        Local1 = \_SB.PC00.TRP2.CRPM ()
                        If ((Local1 != 0xFF))
                        {
                            Local0 |= Local1
                        }

                        Local1 = \_SB.PC00.TRP3.CRPM ()
                        If ((Local1 != 0xFF))
                        {
                            Local0 |= Local1
                        }

                        If ((Local0 != Zero))
                        {
                            ADBG ("D3 100ms delay as one of iTBT RP is in D3 state")
                            Sleep (0x64)
                            \_SB.PC00.TRP0.HPEV ()
                            \_SB.PC00.TRP1.HPEV ()
                            \_SB.PC00.TRP2.HPEV ()
                            \_SB.PC00.TRP3.HPEV ()
                        }

                        \_SB.PC00.TRP0.LTRC ()
                        \_SB.PC00.TRP1.LTRC ()
                        \_SB.PC00.TRP2.LTRC ()
                        \_SB.PC00.TRP3.LTRC ()
                    }
                }
            }
        }

        Method (SL69, 0, NotSerialized)
        {
            If (CondRefOf (\_SB.PC00.TXHC))
            {
                If ((\_SB.PC00.TRP0.HPME () == One))
                {
                    ADBG ("Wake TBT RP0 and DMA")
                    Notify (\_SB.PC00.TDM0, 0x02) // Device Wake
                    Notify (\_SB.PC00.TRP0, 0x02) // Device Wake
                }

                If ((\_SB.PC00.TRP1.HPME () == One))
                {
                    ADBG ("Wake TBT RP1 and DMA")
                    Notify (\_SB.PC00.TDM0, 0x02) // Device Wake
                    Notify (\_SB.PC00.TRP1, 0x02) // Device Wake
                }

                If ((\_SB.PC00.TRP2.HPME () == One))
                {
                    ADBG ("Wake TBT RP2 and DMA")
                    Notify (\_SB.PC00.TDM1, 0x02) // Device Wake
                    Notify (\_SB.PC00.TRP2, 0x02) // Device Wake
                }

                If ((\_SB.PC00.TRP3.HPME () == One))
                {
                    ADBG ("Wake TBT RP3 and DMA")
                    Notify (\_SB.PC00.TDM1, 0x02) // Device Wake
                    Notify (\_SB.PC00.TRP3, 0x02) // Device Wake
                }
            }

            If (CondRefOf (GP1E))
            {
                If ((GP1E == Zero))
                {
                    \_SB.PC00.RP01.HPME ()
                    \_SB.PC00.RP02.HPME ()
                    \_SB.PC00.RP03.HPME ()
                    \_SB.PC00.RP04.HPME ()
                    \_SB.PC00.RP05.HPME ()
                    \_SB.PC00.RP06.HPME ()
                    \_SB.PC00.RP07.HPME ()
                    \_SB.PC00.RP08.HPME ()
                    \_SB.PC00.RP09.HPME ()
                    \_SB.PC00.RP10.HPME ()
                    \_SB.PC00.RP11.HPME ()
                    \_SB.PC00.RP12.HPME ()
                }
            }
        }

        Method (SL6F, 0, NotSerialized)
        {
            If ((PGRT == One)){}
        }

        Method (SL71, 0, NotSerialized)
        {
            If (CondRefOf (\_SB.PC00.IMNG))
            {
                ADBG ("SL71 - Check if iGfx is in D3 and notify iGfx")
                \_SB.PC00.IMNG (0x71)
            }
        }

        Method (SLA7, 0, NotSerialized)
        {
            If (CondRefOf (\_SB.PC00.TXHC))
            {
                ADBG ("SLA7 - Check HPCX and notify OS first")
                Local0 = Zero
                Local1 = \_SB.PC00.TRP0.CRPM ()
                If ((Local1 != 0xFF))
                {
                    Local0 |= Local1
                }

                Local1 = \_SB.PC00.TRP1.CRPM ()
                If ((Local1 != 0xFF))
                {
                    Local0 |= Local1
                }

                Local1 = \_SB.PC00.TRP2.CRPM ()
                If ((Local1 != 0xFF))
                {
                    Local0 |= Local1
                }

                Local1 = \_SB.PC00.TRP3.CRPM ()
                If ((Local1 != 0xFF))
                {
                    Local0 |= Local1
                }

                If ((Local0 != Zero))
                {
                    ADBG ("D3 100ms delay as one of iTBT RP is in D3 state")
                    Sleep (0x64)
                    \_SB.PC00.TRP0.HPEV ()
                    \_SB.PC00.TRP1.HPEV ()
                    \_SB.PC00.TRP2.HPEV ()
                    \_SB.PC00.TRP3.HPEV ()
                }

                \_SB.PC00.TRP0.LTRC ()
                \_SB.PC00.TRP1.LTRC ()
                \_SB.PC00.TRP2.LTRC ()
                \_SB.PC00.TRP3.LTRC ()
            }
        }

        Method (SLC7, 0, NotSerialized)
        {
            If (CondRefOf (\_SB.PC00.TXHC))
            {
                If ((\_SB.PC00.TRP0.HPME () == One))
                {
                    ADBG ("Wake TBT RP0 and DMA")
                    Notify (\_SB.PC00.TDM0, 0x02) // Device Wake
                    Notify (\_SB.PC00.TRP0, 0x02) // Device Wake
                }

                If ((\_SB.PC00.TRP1.HPME () == One))
                {
                    ADBG ("Wake TBT RP1 and DMA")
                    Notify (\_SB.PC00.TDM0, 0x02) // Device Wake
                    Notify (\_SB.PC00.TRP1, 0x02) // Device Wake
                }

                If ((\_SB.PC00.TRP2.HPME () == One))
                {
                    ADBG ("Wake TBT RP2 and DMA")
                    Notify (\_SB.PC00.TDM1, 0x02) // Device Wake
                    Notify (\_SB.PC00.TRP2, 0x02) // Device Wake
                }

                If ((\_SB.PC00.TRP3.HPME () == One))
                {
                    ADBG ("Wake TBT RP3 and DMA")
                    Notify (\_SB.PC00.TDM1, 0x02) // Device Wake
                    Notify (\_SB.PC00.TRP3, 0x02) // Device Wake
                }
            }

            \_SB.PC00.RP10.HPME ()
            \_SB.PC00.RP11.HPME ()
            \_SB.PC00.RP12.HPME ()
        }

        Method (SLA0, 0, NotSerialized)
        {
            Sleep (0x64)
            If (CondRefOf (\_SB.PC00.RP01))
            {
                If (((\_SB.PC00.RP01.VDID != 0xFFFFFFFF) && \_SB.PC00.RP01.HPSX))
                {
                    If (\_SB.PC00.RP01.PDCX)
                    {
                        \_SB.PC00.RP01.PDCX = One
                        \_SB.PC00.RP01.HPSX = One
                        If (!\_SB.PC00.RP01.PDSX)
                        {
                            \_SB.PC00.RP01.L0SE = Zero
                        }

                        Notify (\_SB.PC00.RP01, Zero) // Bus Check
                    }
                    Else
                    {
                        \_SB.PC00.RP01.HPSX = One
                    }
                }
            }

            If (CondRefOf (\_SB.PC00.RP02))
            {
                If (((\_SB.PC00.RP02.VDID != 0xFFFFFFFF) && \_SB.PC00.RP02.HPSX))
                {
                    If (\_SB.PC00.RP02.PDCX)
                    {
                        \_SB.PC00.RP02.PDCX = One
                        \_SB.PC00.RP02.HPSX = One
                        If (!\_SB.PC00.RP02.PDSX)
                        {
                            \_SB.PC00.RP02.L0SE = Zero
                        }

                        Notify (\_SB.PC00.RP02, Zero) // Bus Check
                    }
                    Else
                    {
                        \_SB.PC00.RP02.HPSX = One
                    }
                }
            }

            If (CondRefOf (\_SB.PC00.RP03))
            {
                If (((\_SB.PC00.RP03.VDID != 0xFFFFFFFF) && \_SB.PC00.RP03.HPSX))
                {
                    If (\_SB.PC00.RP03.PDCX)
                    {
                        \_SB.PC00.RP03.PDCX = One
                        \_SB.PC00.RP03.HPSX = One
                        If (!\_SB.PC00.RP03.PDSX)
                        {
                            \_SB.PC00.RP03.L0SE = Zero
                        }

                        Notify (\_SB.PC00.RP03, Zero) // Bus Check
                    }
                    Else
                    {
                        \_SB.PC00.RP03.HPSX = One
                    }
                }
            }

            If (CondRefOf (\_SB.PC00.RP04))
            {
                If (((\_SB.PC00.RP04.VDID != 0xFFFFFFFF) && \_SB.PC00.RP04.HPSX))
                {
                    If (\_SB.PC00.RP04.PDCX)
                    {
                        \_SB.PC00.RP04.PDCX = One
                        \_SB.PC00.RP04.HPSX = One
                        If (!\_SB.PC00.RP04.PDSX)
                        {
                            \_SB.PC00.RP04.L0SE = Zero
                        }

                        Notify (\_SB.PC00.RP04, Zero) // Bus Check
                    }
                    Else
                    {
                        \_SB.PC00.RP04.HPSX = One
                    }
                }
            }
        }

        Method (SLA1, 0, NotSerialized)
        {
            Sleep (0x64)
            If (CondRefOf (\_SB.PC00.RP05))
            {
                If (((\_SB.PC00.RP05.VDID != 0xFFFFFFFF) && \_SB.PC00.RP05.HPSX))
                {
                    If (\_SB.PC00.RP05.PDCX)
                    {
                        \_SB.PC00.RP05.PDCX = One
                        \_SB.PC00.RP05.HPSX = One
                        If (!\_SB.PC00.RP05.PDSX)
                        {
                            \_SB.PC00.RP05.L0SE = Zero
                        }

                        Notify (\_SB.PC00.RP05, Zero) // Bus Check
                    }
                    Else
                    {
                        \_SB.PC00.RP05.HPSX = One
                    }
                }
            }

            If (CondRefOf (\_SB.PC00.RP06))
            {
                If (((\_SB.PC00.RP06.VDID != 0xFFFFFFFF) && \_SB.PC00.RP06.HPSX))
                {
                    If (\_SB.PC00.RP06.PDCX)
                    {
                        \_SB.PC00.RP06.PDCX = One
                        \_SB.PC00.RP06.HPSX = One
                        If (!\_SB.PC00.RP06.PDSX)
                        {
                            \_SB.PC00.RP06.L0SE = Zero
                        }

                        Notify (\_SB.PC00.RP06, Zero) // Bus Check
                    }
                    Else
                    {
                        \_SB.PC00.RP06.HPSX = One
                    }
                }
            }

            If (CondRefOf (\_SB.PC00.RP07))
            {
                If (((\_SB.PC00.RP07.VDID != 0xFFFFFFFF) && \_SB.PC00.RP07.HPSX))
                {
                    If (\_SB.PC00.RP07.PDCX)
                    {
                        \_SB.PC00.RP07.PDCX = One
                        \_SB.PC00.RP07.HPSX = One
                        If (!\_SB.PC00.RP07.PDSX)
                        {
                            \_SB.PC00.RP07.L0SE = Zero
                        }

                        Notify (\_SB.PC00.RP07, Zero) // Bus Check
                    }
                    Else
                    {
                        \_SB.PC00.RP07.HPSX = One
                    }
                }
            }

            If (CondRefOf (\_SB.PC00.RP08))
            {
                If (((\_SB.PC00.RP08.VDID != 0xFFFFFFFF) && \_SB.PC00.RP08.HPSX))
                {
                    If (\_SB.PC00.RP08.PDCX)
                    {
                        \_SB.PC00.RP08.PDCX = One
                        \_SB.PC00.RP08.HPSX = One
                        If (!\_SB.PC00.RP08.PDSX)
                        {
                            \_SB.PC00.RP08.L0SE = Zero
                        }

                        Notify (\_SB.PC00.RP08, Zero) // Bus Check
                    }
                    Else
                    {
                        \_SB.PC00.RP08.HPSX = One
                    }
                }
            }
        }

        Method (SLA2, 0, NotSerialized)
        {
            Sleep (0x64)
            If (CondRefOf (\_SB.PC00.RP09))
            {
                If (((\_SB.PC00.RP09.VDID != 0xFFFFFFFF) && \_SB.PC00.RP09.HPSX))
                {
                    If (\_SB.PC00.RP09.PDCX)
                    {
                        \_SB.PC00.RP09.PDCX = One
                        \_SB.PC00.RP09.HPSX = One
                        If (!\_SB.PC00.RP09.PDSX)
                        {
                            \_SB.PC00.RP09.L0SE = Zero
                        }

                        Notify (\_SB.PC00.RP09, Zero) // Bus Check
                    }
                    Else
                    {
                        \_SB.PC00.RP09.HPSX = One
                    }
                }
            }
        }

        Method (SLC0, 0, NotSerialized)
        {
            \_SB.PC00.RP01.HPME ()
            \_SB.PC00.RP02.HPME ()
            \_SB.PC00.RP03.HPME ()
            \_SB.PC00.RP04.HPME ()
        }

        Method (SLC1, 0, NotSerialized)
        {
            \_SB.PC00.RP05.HPME ()
            \_SB.PC00.RP06.HPME ()
            \_SB.PC00.RP07.HPME ()
            \_SB.PC00.RP08.HPME ()
        }

        Method (SLC2, 0, NotSerialized)
        {
            \_SB.PC00.RP09.HPME ()
        }
    }
}

