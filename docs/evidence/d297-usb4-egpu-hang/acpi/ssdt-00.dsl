/*
 * Intel ACPI Component Architecture
 * AML/ASL+ Disassembler version 20251212 (64-bit version)
 * Copyright (c) 2000 - 2025 Intel Corporation
 * 
 * Disassembling to symbolic ASL+ operators
 *
 * Disassembly of ssdt-00.aml
 *
 * Original Table Header:
 *     Signature        "SSDT"
 *     Length           0x0000038C (908)
 *     Revision         0x02
 *     Checksum         0x23
 *     OEM ID           "LENOVO"
 *     OEM Table ID     "Pmax_Dev"
 *     OEM Revision     0x00000001 (1)
 *     Compiler ID      "INTL"
 *     Compiler Version 0x20210930 (539035952)
 */
DefinitionBlock ("", "SSDT", 2, "LENOVO", "Pmax_Dev", 0x00000001)
{
    External (PXAC, IntObj)
    External (PXDC, IntObj)
    External (PXFD, IntObj)
    External (PXUC, IntObj)
    External (PXWC, IntObj)

    Scope (\_SB)
    {
        Device (PMAX)
        {
            Name (_HID, "INT3533")  // _HID: Hardware ID
            Name (RTKS, "\\_SB.PC00.HDAS.RTKC")
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Name (RTK0, 0x0BB8)
            Name (RTKX, 0x0000)
            Name (WFCS, "\\_SB.PC00.LNK2")
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Name (WFC0, 0x032A)
            Name (WFCX, 0x0000)
            Name (UFCS, "\\_SB.PC00.LNK0")
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Name (UFC0, 0x033E)
            Name (UFCX, 0x0000)
            Name (FLDS, "\\_SB.PC00.CLP0")
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Noop
            Name (FLD0, 0x2328)
            Name (FLDX, 0x0000)
            Name (DSMP, Package (0x04)
            {
                Package (0x02)
                {
                    "", 
                    Package (0x02)
                    {
                        Package (0x02)
                        {
                            Zero, 
                            0xFFFF
                        }, 

                        Package (0x02)
                        {
                            One, 
                            0xFFFF
                        }
                    }
                }, 

                Package (0x02)
                {
                    "", 
                    Package (0x02)
                    {
                        Package (0x02)
                        {
                            Zero, 
                            0xFFFF
                        }, 

                        Package (0x02)
                        {
                            One, 
                            0xFFFF
                        }
                    }
                }, 

                Package (0x02)
                {
                    "", 
                    Package (0x02)
                    {
                        Package (0x02)
                        {
                            Zero, 
                            0xFFFF
                        }, 

                        Package (0x02)
                        {
                            One, 
                            0xFFFF
                        }
                    }
                }, 

                Package (0x02)
                {
                    "", 
                    Package (0x02)
                    {
                        Package (0x02)
                        {
                            Zero, 
                            0xFFFF
                        }, 

                        Package (0x02)
                        {
                            One, 
                            0xFFFF
                        }
                    }
                }
            })
            Method (_DSM, 4, NotSerialized)  // _DSM: Device-Specific Method
            {
                If ((Arg0 == ToUUID ("2256364f-a3a9-4c9a-bcec-a34a34a57144") /* Unknown UUID */))
                {
                    DerefOf (DSMP [Zero]) [Zero] = RTKS /* \_SB_.PMAX.RTKS */
                    DerefOf (DerefOf (DerefOf (DSMP [Zero]) [One]
                        ) [Zero]) [One] = RTK0 /* \_SB_.PMAX.RTK0 */
                    DerefOf (DerefOf (DerefOf (DSMP [Zero]) [One]
                        ) [One]) [One] = RTKX /* \_SB_.PMAX.RTKX */
                    DerefOf (DSMP [One]) [Zero] = WFCS /* \_SB_.PMAX.WFCS */
                    DerefOf (DerefOf (DerefOf (DSMP [One]) [One]
                        ) [Zero]) [One] = WFC0 /* \_SB_.PMAX.WFC0 */
                    DerefOf (DerefOf (DerefOf (DSMP [One]) [One]
                        ) [One]) [One] = WFCX /* \_SB_.PMAX.WFCX */
                    DerefOf (DSMP [0x02]) [Zero] = UFCS /* \_SB_.PMAX.UFCS */
                    DerefOf (DerefOf (DerefOf (DSMP [0x02]) [One]
                        ) [Zero]) [One] = UFC0 /* \_SB_.PMAX.UFC0 */
                    DerefOf (DerefOf (DerefOf (DSMP [0x02]) [One]
                        ) [One]) [One] = UFCX /* \_SB_.PMAX.UFCX */
                    DerefOf (DSMP [0x03]) [Zero] = FLDS /* \_SB_.PMAX.FLDS */
                    DerefOf (DerefOf (DerefOf (DSMP [0x03]) [One]
                        ) [Zero]) [One] = FLD0 /* \_SB_.PMAX.FLD0 */
                    DerefOf (DerefOf (DerefOf (DSMP [0x03]) [One]
                        ) [One]) [One] = FLDX /* \_SB_.PMAX.FLDX */
                    If ((Arg1 == Zero))
                    {
                        If ((Arg2 == Zero))
                        {
                            Return (Buffer (One)
                            {
                                 0x03                                             // .
                            })
                        }

                        If ((Arg2 == One))
                        {
                            If ((PXAC == One))
                            {
                                DerefOf (DSMP [Zero]) [One] = Package (0x02)
                                    {
                                        Package (0x02)
                                        {
                                            Zero, 
                                            Zero
                                        }, 

                                        Package (0x02)
                                        {
                                            One, 
                                            Zero
                                        }
                                    }
                            }

                            If ((PXWC == One))
                            {
                                DerefOf (DSMP [One]) [One] = Package (0x02)
                                    {
                                        Package (0x02)
                                        {
                                            Zero, 
                                            Zero
                                        }, 

                                        Package (0x02)
                                        {
                                            One, 
                                            Zero
                                        }
                                    }
                            }

                            If ((PXUC == One))
                            {
                                DerefOf (DSMP [0x02]) [One] = Package (0x02)
                                    {
                                        Package (0x02)
                                        {
                                            Zero, 
                                            Zero
                                        }, 

                                        Package (0x02)
                                        {
                                            One, 
                                            Zero
                                        }
                                    }
                            }

                            If ((PXFD == One))
                            {
                                DerefOf (DSMP [0x03]) [One] = Package (0x02)
                                    {
                                        Package (0x02)
                                        {
                                            Zero, 
                                            Zero
                                        }, 

                                        Package (0x02)
                                        {
                                            One, 
                                            Zero
                                        }
                                    }
                            }

                            Return (DSMP) /* \_SB_.PMAX.DSMP */
                        }
                    }

                    Return (Zero)
                }

                Return (Zero)
            }

            Method (_STA, 0, NotSerialized)  // _STA: Status
            {
                If ((PXDC == Zero))
                {
                    Return (Zero)
                }
                Else
                {
                    Return (0x0F)
                }
            }
        }
    }
}

