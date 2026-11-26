/*
 * Intel ACPI Component Architecture
 * AML/ASL+ Disassembler version 20251212 (64-bit version)
 * Copyright (c) 2000 - 2025 Intel Corporation
 * 
 * Disassembling to symbolic ASL+ operators
 *
 * Disassembly of ssdt-12.aml
 *
 * Original Table Header:
 *     Signature        "SSDT"
 *     Length           0x00005E42 (24130)
 *     Revision         0x02
 *     Checksum         0x03
 *     OEM ID           "LENOVO"
 *     OEM Table ID     "TbtTypeC"
 *     OEM Revision     0x00000000 (0)
 *     Compiler ID      "INTL"
 *     Compiler Version 0x20210930 (539035952)
 */
DefinitionBlock ("", "SSDT", 2, "LENOVO", "TbtTypeC", 0x00000000)
{
    External (_SB_.PC00.RP01.PXSX, DeviceObj)
    External (_SB_.PC00.RP05.PXSX, DeviceObj)
    External (_SB_.PC00.RP09.DTWK, MethodObj)    // 1 Arguments
    External (_SB_.PC00.RP09.PXSX, DeviceObj)
    External (_SB_.PC00.RP09.PXSX.SLOT, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TBDH.NHI4, DeviceObj)
    External (_SB_.PC00.RP09.PXSX.TBDH.NHI5, DeviceObj)
    External (_SB_.PC00.RP09.PXSX.TBDH.SLOT, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP1C, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP1D, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP1T, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP2C, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP2D, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP2T, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP3C, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP3D, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP3T, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP4C, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP4D, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP4T, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP5C, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP5D, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP5T, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP6C, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP6D, UnknownObj)
    External (_SB_.PC00.RP09.PXSX.TP6T, UnknownObj)
    External (_SB_.PC00.RP09.TUID, IntObj)
    External (_SB_.PC00.RP09.WAKG, IntObj)
    External (_SB_.PC00.RP10.DTWK, MethodObj)    // 1 Arguments
    External (_SB_.PC00.RP10.PXSX, DeviceObj)
    External (_SB_.PC00.RP10.PXSX.SLOT, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TBDH.NHI4, DeviceObj)
    External (_SB_.PC00.RP10.PXSX.TBDH.NHI5, DeviceObj)
    External (_SB_.PC00.RP10.PXSX.TBDH.SLOT, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP1C, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP1D, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP1T, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP2C, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP2D, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP2T, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP3C, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP3D, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP3T, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP4C, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP4D, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP4T, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP5C, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP5D, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP5T, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP6C, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP6D, UnknownObj)
    External (_SB_.PC00.RP10.PXSX.TP6T, UnknownObj)
    External (_SB_.PC00.RP10.TUID, IntObj)
    External (_SB_.PC00.RP10.WAKG, IntObj)
    External (_SB_.PC00.RP17.DTWK, MethodObj)    // 1 Arguments
    External (_SB_.PC00.RP17.PXSX, DeviceObj)
    External (_SB_.PC00.RP17.PXSX.SLOT, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TBDH.NHI4, DeviceObj)
    External (_SB_.PC00.RP17.PXSX.TBDH.NHI5, DeviceObj)
    External (_SB_.PC00.RP17.PXSX.TBDH.SLOT, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP1C, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP1D, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP1T, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP2C, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP2D, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP2T, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP3C, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP3D, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP3T, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP4C, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP4D, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP4T, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP5C, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP5D, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP5T, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP6C, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP6D, UnknownObj)
    External (_SB_.PC00.RP17.PXSX.TP6T, UnknownObj)
    External (_SB_.PC00.RP17.TUID, IntObj)
    External (_SB_.PC00.RP17.WAKG, IntObj)
    External (_SB_.PC02.RP09.DTWK, MethodObj)    // 1 Arguments
    External (_SB_.PC02.RP09.PXSX, DeviceObj)
    External (_SB_.PC02.RP09.PXSX.SLOT, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TBDH.NHI4, DeviceObj)
    External (_SB_.PC02.RP09.PXSX.TBDH.NHI5, DeviceObj)
    External (_SB_.PC02.RP09.PXSX.TBDH.SLOT, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP1C, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP1D, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP1T, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP2C, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP2D, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP2T, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP3C, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP3D, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP3T, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP4C, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP4D, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP4T, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP5C, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP5D, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP5T, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP6C, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP6D, UnknownObj)
    External (_SB_.PC02.RP09.PXSX.TP6T, UnknownObj)
    External (_SB_.PC02.RP09.TUID, IntObj)
    External (_SB_.PC02.RP09.WAKG, IntObj)
    External (_SB_.UBTC.RUCC, MethodObj)    // 3 Arguments
    External (ADBG, MethodObj)    // 1 Arguments
    External (DPM1, IntObj)
    External (DPM2, IntObj)
    External (DPM3, IntObj)
    External (DSCE, IntObj)
    External (DTFS, IntObj)
    External (DTMB, MethodObj)    // 4 Arguments
    External (DTRT, IntObj)
    External (NDUS, IntObj)
    External (NTUS, IntObj)
    External (RPS0, IntObj)
    External (RPS1, IntObj)
    External (RPT0, IntObj)
    External (RPT1, IntObj)
    External (TBSE, IntObj)
    External (TP1D, IntObj)
    External (TP1P, IntObj)
    External (TP1T, IntObj)
    External (TP2D, IntObj)
    External (TP2P, IntObj)
    External (TP2T, IntObj)
    External (TP3C, IntObj)
    External (TP3D, IntObj)
    External (TP3P, IntObj)
    External (TP3T, IntObj)
    External (TP4D, IntObj)
    External (TP4P, IntObj)
    External (TP4T, IntObj)
    External (TP5C, IntObj)
    External (TP5D, IntObj)
    External (TP5P, IntObj)
    External (TP5T, IntObj)
    External (TP6D, IntObj)
    External (TP6P, IntObj)
    External (TP6T, IntObj)

    If (CondRefOf (\RPS0))
    {
        ADBG (Concatenate ("upstream port RPS0 = ", ToHexString (RPS0)))
    }

    If (CondRefOf (\RPS1))
    {
        ADBG (Concatenate ("upstream port RPS1 = ", ToHexString (RPS1)))
    }

    If (CondRefOf (\RPT0))
    {
        ADBG (Concatenate ("upstream port RPT0 = ", ToHexString (RPT0)))
    }

    If (CondRefOf (\RPT1))
    {
        ADBG (Concatenate ("upstream port RPT1 = ", ToHexString (RPT1)))
    }

    If (CondRefOf (\DTFS))
    {
        If (((DTFS == One) && (RPS0 == 0x02)))
        {
            If ((RPT0 == 0x02))
            {
                Scope (\_SB.PC00.RP09.PXSX)
                {
                    Name (TURP, 0x09)
                    Name (RPSN, One)
                    Device (TBDU)
                    {
                        Name (_ADR, 0x00020000)  // _ADR: Address
                        Device (XHCI)
                        {
                            Name (_ADR, Zero)  // _ADR: Address
                            Method (_S0W, 0, NotSerialized)  // _S0W: S0 Device Wake State
                            {
                                ADBG (Concatenate ("[dTBT] DTBT XHCI S0W - RP SLOT ", ToHexString (SLOT)))
                                Return (0x04)
                            }

                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                            {
                                ADBG (Concatenate ("[dTBT] DTBT XHCI PS0 - RP SLOT ", ToHexString (SLOT)))
                            }

                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                            {
                                ADBG (Concatenate ("[dTBT] DTBT XHCI PS3 - RP SLOT ", ToHexString (SLOT)))
                            }

                            Device (RHUB)
                            {
                                Name (_ADR, Zero)  // _ADR: Address
                                Method (_DSM, 4, Serialized)  // _DSM: Device-Specific Method
                                {
                                    ADBG ("[dTBT] XHCI RHUB _DSM")
                                    If ((Arg0 == ToUUID ("ce2ee385-00e6-48cb-9f05-2edb927c4899") /* USB Controller */))
                                    {
                                        If ((Arg2 == Zero))
                                        {
                                            ADBG ("[dTBT] XHCI RHUB F0")
                                            Return (Buffer (One)
                                            {
                                                 0x81                                             // .
                                            })
                                        }

                                        If ((Arg2 == 0x07))
                                        {
                                            ADBG ("[dTBT] XHCI RHUB F7")
                                            Return (Buffer (One)
                                            {
                                                 0x01                                             // .
                                            })
                                        }
                                    }

                                    Return (Buffer (One)
                                    {
                                         0x00                                             // .
                                    })
                                }

                                Method (_S0W, 0, NotSerialized)  // _S0W: S0 Device Wake State
                                {
                                    ADBG (Concatenate ("[dTBT] DTBT XHCI RHUB S0W return 4 - RP SLOT ", ToHexString (SLOT)))
                                    Return (0x04)
                                }

                                Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                                {
                                    ADBG (Concatenate ("[dTBT] DTBT XHCI RHUB PS0 - RP SLOT ", ToHexString (SLOT)))
                                }

                                Method (_PS2, 0, Serialized)  // _PS2: Power State 2
                                {
                                    ADBG (Concatenate ("[dTBT] DTBT XHCI RHUB PS2 - RP SLOT ", ToHexString (SLOT)))
                                }

                                Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                                {
                                    ADBG (Concatenate ("[dTBT] DTBT XHCI RHUB PS3 - RP SLOT ", ToHexString (SLOT)))
                                }

                                Method (SLMS, 1, Serialized)
                                {
                                    Local0 = (TURP << 0x02)
                                    Local0 |= One
                                    If ((((TP1D >> One) == Local0) && (Arg0 == TP1T)))
                                    {
                                        Return ((TP1D & One))
                                    }
                                    ElseIf ((((TP2D >> One) == Local0) && (Arg0 == TP2T)))
                                    {
                                        Return ((TP2D & One))
                                    }
                                    ElseIf ((((TP3D >> One) == Local0) && (Arg0 == TP3T)))
                                    {
                                        Return ((TP3D & One))
                                    }
                                    ElseIf ((((TP4D >> One) == Local0) && (Arg0 == TP4T)))
                                    {
                                        Return ((TP4D & One))
                                    }
                                    ElseIf ((((TP5D >> One) == Local0) && (Arg0 == TP5T)))
                                    {
                                        Return ((TP5D & One))
                                    }
                                    ElseIf ((((TP6D >> One) == Local0) && (Arg0 == TP6T)))
                                    {
                                        Return ((TP6D & One))
                                    }
                                    Else
                                    {
                                        Return (Zero)
                                    }
                                }

                                Method (FPCP, 2, Serialized)
                                {
                                    Local0 = (TURP << 0x02)
                                    Local0 |= One
                                    If ((((TP1D >> One) == Local0) && (Arg0 == TP1T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (One, Arg1, TP1C))
                                    }
                                    ElseIf ((((TP2D >> One) == Local0) && (Arg0 == TP2T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x02, Arg1, TP2C))
                                    }
                                    ElseIf ((((TP3D >> One) == Local0) && (Arg0 == TP3T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x03, Arg1, TP3C))
                                    }
                                    ElseIf ((((TP4D >> One) == Local0) && (Arg0 == TP4T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x04, Arg1, TP4C))
                                    }
                                    ElseIf ((((TP5D >> One) == Local0) && (Arg0 == TP5T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x05, Arg1, TP5C))
                                    }
                                    ElseIf ((((TP6D >> One) == Local0) && (Arg0 == TP6T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x06, Arg1, TP6C))
                                    }
                                    ElseIf ((Arg1 == One))
                                    {
                                        Return (TUPC (Zero, Zero))
                                    }
                                    Else
                                    {
                                        Return (TPLD (Zero, Zero))
                                    }
                                }

                                Method (FPSP, 1, Serialized)
                                {
                                    Local0 = (TURP << 0x02)
                                    Local0 |= One
                                    If ((((TP1D >> One) == Local0) && (Arg0 == TP1T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP2D >> One) == Local0) && (Arg0 == TP2T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP3D >> One) == Local0) && (Arg0 == TP3T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP4D >> One) == Local0) && (Arg0 == TP4T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP5D >> One) == Local0) && (Arg0 == TP5T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP6D >> One) == Local0) && (Arg0 == TP6T)))
                                    {
                                        Return (One)
                                    }
                                    Else
                                    {
                                        Return (Zero)
                                    }
                                }

                                Method (TPLD, 2, Serialized)
                                {
                                    Name (PCKG, Package (0x01)
                                    {
                                        Buffer (0x10){}
                                    })
                                    CreateField (DerefOf (PCKG [Zero]), Zero, 0x07, REV)
                                    REV = One
                                    CreateField (DerefOf (PCKG [Zero]), 0x40, One, VISI)
                                    VISI = Arg0
                                    CreateField (DerefOf (PCKG [Zero]), 0x57, 0x08, GPOS)
                                    GPOS = Arg1
                                    CreateField (DerefOf (PCKG [Zero]), 0x4A, 0x04, SHAP)
                                    SHAP = One
                                    CreateField (DerefOf (PCKG [Zero]), 0x20, 0x10, WID)
                                    WID = 0x08
                                    CreateField (DerefOf (PCKG [Zero]), 0x30, 0x10, HGT)
                                    HGT = 0x03
                                    Return (PCKG) /* \_SB_.PC00.RP09.PXSX.TBDU.XHCI.RHUB.TPLD.PCKG */
                                }

                                Method (TUPC, 2, Serialized)
                                {
                                    Name (PCKG, Package (0x04)
                                    {
                                        One, 
                                        Zero, 
                                        Zero, 
                                        Zero
                                    })
                                    PCKG [Zero] = Arg0
                                    PCKG [One] = Arg1
                                    If ((RPSN == One))
                                    {
                                        PCKG [0x02] = TP3C /* External reference */
                                    }
                                    ElseIf ((RPSN == 0x02))
                                    {
                                        PCKG [0x02] = TP5C /* External reference */
                                    }

                                    Return (PCKG) /* \_SB_.PC00.RP09.PXSX.TBDU.XHCI.RHUB.TUPC.PCKG */
                                }

                                If ((FPSP (One) == One))
                                {
                                    Device (HS01)
                                    {
                                        Name (_ADR, One)  // _ADR: Address
                                        Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                                        {
                                            If ((SLMS (One) == Zero))
                                            {
                                                Return (FPCP (One, One))
                                            }
                                            Else
                                            {
                                                Return (TUPC (Zero, Zero))
                                            }
                                        }

                                        Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                                        {
                                            If ((SLMS (One) == Zero))
                                            {
                                                Return (FPCP (One, 0x02))
                                            }
                                            Else
                                            {
                                                Return (TPLD (Zero, Zero))
                                            }
                                        }
                                    }
                                }

                                If ((FPSP (0x02) == One))
                                {
                                    Device (HS02)
                                    {
                                        Name (_ADR, 0x02)  // _ADR: Address
                                        Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                                        {
                                            If ((SLMS (0x02) == Zero))
                                            {
                                                Return (FPCP (0x02, One))
                                            }
                                            Else
                                            {
                                                Return (TUPC (Zero, Zero))
                                            }
                                        }

                                        Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                                        {
                                            If ((SLMS (0x02) == Zero))
                                            {
                                                Return (FPCP (0x02, 0x02))
                                            }
                                            Else
                                            {
                                                Return (TPLD (Zero, Zero))
                                            }
                                        }
                                    }
                                }

                                If ((FPSP (One) == One))
                                {
                                    Device (SS01)
                                    {
                                        Name (_ADR, 0x03)  // _ADR: Address
                                        Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                                        {
                                            Return (FPCP (One, One))
                                        }

                                        Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                                        {
                                            Return (FPCP (One, 0x02))
                                        }

                                        If (CondRefOf (\_SB.PC00.RP09.TUID))
                                        {
                                            ADBG (Concatenate ("[dTBT] DTBT SS01 - RP SLOT ", ToHexString (SLOT)))
                                            If ((\_SB.PC00.RP09.TUID <= One))
                                            {
                                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                                {
                                                    ADBG (Concatenate ("[dTBT] DTBT SS01 DSD - RP SLOT ", ToHexString (SLOT)))
                                                    If ((\_SB.PC00.RP09.TUID == Zero))
                                                    {
                                                        Return (Package (0x02)
                                                        {
                                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                                            Package (0x02)
                                                            {
                                                                Package (0x02)
                                                                {
                                                                    "usb4-host-interface", 
                                                                    \_SB.PC00.RP09.PXSX.TBDH.NHI4
                                                                }, 

                                                                Package (0x02)
                                                                {
                                                                    "usb4-port-number", 
                                                                    Zero
                                                                }
                                                            }
                                                        })
                                                    }
                                                    Else
                                                    {
                                                        Return (Package (0x02)
                                                        {
                                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                                            Package (0x02)
                                                            {
                                                                Package (0x02)
                                                                {
                                                                    "usb4-host-interface", 
                                                                    \_SB.PC00.RP09.PXSX.TBDH.NHI5
                                                                }, 

                                                                Package (0x02)
                                                                {
                                                                    "usb4-port-number", 
                                                                    Zero
                                                                }
                                                            }
                                                        })
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                If ((FPSP (0x02) == One))
                                {
                                    Device (SS02)
                                    {
                                        Name (_ADR, 0x04)  // _ADR: Address
                                        Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                                        {
                                            Return (FPCP (0x02, One))
                                        }

                                        Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                                        {
                                            Return (FPCP (0x02, 0x02))
                                        }

                                        If (CondRefOf (\_SB.PC00.RP09.TUID))
                                        {
                                            ADBG (Concatenate ("[dTBT] DTBT SS02 - RP SLOT ", ToHexString (SLOT)))
                                            If ((\_SB.PC00.RP09.TUID <= One))
                                            {
                                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                                {
                                                    ADBG (Concatenate ("[dTBT] DTBT SS02 DSD - RP SLOT ", ToHexString (SLOT)))
                                                    If ((\_SB.PC00.RP09.TUID == Zero))
                                                    {
                                                        Return (Package (0x02)
                                                        {
                                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                                            Package (0x02)
                                                            {
                                                                Package (0x02)
                                                                {
                                                                    "usb4-host-interface", 
                                                                    \_SB.PC00.RP09.PXSX.TBDH.NHI4
                                                                }, 

                                                                Package (0x02)
                                                                {
                                                                    "usb4-port-number", 
                                                                    One
                                                                }
                                                            }
                                                        })
                                                    }
                                                    Else
                                                    {
                                                        Return (Package (0x02)
                                                        {
                                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                                            Package (0x02)
                                                            {
                                                                Package (0x02)
                                                                {
                                                                    "usb4-host-interface", 
                                                                    \_SB.PC00.RP09.PXSX.TBDH.NHI5
                                                                }, 

                                                                Package (0x02)
                                                                {
                                                                    "usb4-port-number", 
                                                                    One
                                                                }
                                                            }
                                                        })
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            Method (_RMV, 0, NotSerialized)  // _RMV: Removal Status
                            {
                                Return (Zero)
                            }
                        }
                    }

                    If ((CondRefOf (\_SB.PC00.RP09.TUID) && CondRefOf (\DTRT)))
                    {
                        Device (TBDH)
                        {
                            Name (_ADR, Zero)  // _ADR: Address
                            If ((\_SB.PC00.RP09.TUID <= One))
                            {
                                If ((\_SB.PC00.RP09.TUID == Zero))
                                {
                                    Device (NHI4)
                                    {
                                        Name (_ADR, Zero)  // _ADR: Address
                                        If (CondRefOf (\DTRT))
                                        {
                                            Method (_S0W, 0, Serialized)  // _S0W: S0 Device Wake State
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _S0W - RP SLOT ", ToHexString (SLOT)))
                                                If ((\DTRT == Zero))
                                                {
                                                    ADBG ("[dTBT] DTBT NHI S0W return 3")
                                                    Return (0x03)
                                                }
                                                Else
                                                {
                                                    ADBG ("[dTBT] DTBT NHI S0W return 4")
                                                    Return (0x04)
                                                }
                                            }

                                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS0 - RP SLOT ", ToHexString (SLOT)))
                                            }

                                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS3 Start - RP SLOT", ToHexString (SLOT)))
                                                If (!(CondRefOf (\RPS0) && CondRefOf (\RPT0)))
                                                {
                                                    ADBG ("[dTBT] dTBT NHI _PS3 End due to RPS# or RPT# not present!")
                                                    Return (Zero)
                                                }

                                                If (CondRefOf (\_SB.PC00.RP09.WAKG))
                                                {
                                                    ADBG (Concatenate ("[dTBT] WAKG = ", ToHexString (\_SB.PC00.RP09.WAKG)))
                                                    If ((\_SB.PC00.RP09.WAKG != Zero))
                                                    {
                                                        ADBG ("[dTBT] Enable Wake")
                                                        \_SB.PC00.RP09.DTWK (Zero)
                                                    }
                                                }

                                                If (CondRefOf (\DTMB))
                                                {
                                                    \DTMB (0x48, Zero, \RPS0, \RPT0)
                                                }

                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS3 End - RP SLOT ", ToHexString (SLOT)))
                                            }

                                            Method (_RMV, 0, NotSerialized)  // _RMV: Removal Status
                                            {
                                                Return (Zero)
                                            }
                                        }
                                    }
                                }
                                Else
                                {
                                    Device (NHI5)
                                    {
                                        Name (_ADR, Zero)  // _ADR: Address
                                        If (CondRefOf (\DTRT))
                                        {
                                            Method (_S0W, 0, Serialized)  // _S0W: S0 Device Wake State
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _S0W - RP SLOT ", ToHexString (SLOT)))
                                                If ((\DTRT == Zero))
                                                {
                                                    ADBG ("[dTBT] DTBT NHI S0W return 3")
                                                    Return (0x03)
                                                }
                                                Else
                                                {
                                                    ADBG ("[dTBT] DTBT NHI S0W return 4")
                                                    Return (0x04)
                                                }
                                            }

                                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS0 - RP SLOT ", ToHexString (SLOT)))
                                            }

                                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS3 Start - RP SLOT", ToHexString (SLOT)))
                                                If (!(CondRefOf (\RPS1) && CondRefOf (\RPT1)))
                                                {
                                                    ADBG ("[dTBT] dTBT NHI _PS3 End due to RPS# or RPT# not present!")
                                                    Return (Zero)
                                                }

                                                If (CondRefOf (\_SB.PC00.RP09.WAKG))
                                                {
                                                    ADBG (Concatenate ("[dTBT] WAKG = ", ToHexString (\_SB.PC00.RP09.WAKG)))
                                                    If ((\_SB.PC00.RP09.WAKG != Zero))
                                                    {
                                                        ADBG ("[dTBT] Enable Wake")
                                                        \_SB.PC00.RP09.DTWK (Zero)
                                                    }
                                                }

                                                If (CondRefOf (\DTMB))
                                                {
                                                    \DTMB (0x48, Zero, \RPS1, \RPT1)
                                                }

                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS3 End - RP SLOT ", ToHexString (SLOT)))
                                            }

                                            Method (_RMV, 0, NotSerialized)  // _RMV: Removal Status
                                            {
                                                Return (Zero)
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        Device (TBP1)
                        {
                            Name (_ADR, 0x00010000)  // _ADR: Address
                            Method (_S0W, 0, Serialized)  // _S0W: S0 Device Wake State
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS1 _S0W - RP SLOT ", ToHexString (SLOT)))
                                If ((\DTRT == Zero))
                                {
                                    ADBG ("[dTBT] DTBT DS1 S0W return 3")
                                    Return (0x03)
                                }
                                Else
                                {
                                    ADBG ("[dTBT] DTBT DS1 S0W return 4")
                                    Return (0x04)
                                }
                            }

                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS1 _PS0 - RP SLOT ", ToHexString (SLOT)))
                            }

                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS1 _PS3 - RP SLOT ", ToHexString (SLOT)))
                            }

                            If ((\_SB.PC00.RP09.TUID <= One))
                            {
                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                {
                                    ADBG (Concatenate ("[dTBT] dTBT DS1 _DSD - RP SLOT ", ToHexString (SLOT)))
                                    If ((\_SB.PC00.RP09.TUID == Zero))
                                    {
                                        Return (Package (0x02)
                                        {
                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                            Package (0x02)
                                            {
                                                Package (0x02)
                                                {
                                                    "usb4-host-interface", 
                                                    \_SB.PC00.RP09.PXSX.TBDH.NHI4
                                                }, 

                                                Package (0x02)
                                                {
                                                    "usb4-port-number", 
                                                    Zero
                                                }
                                            }
                                        })
                                    }
                                    Else
                                    {
                                        Return (Package (0x02)
                                        {
                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                            Package (0x02)
                                            {
                                                Package (0x02)
                                                {
                                                    "usb4-host-interface", 
                                                    \_SB.PC00.RP09.PXSX.TBDH.NHI5
                                                }, 

                                                Package (0x02)
                                                {
                                                    "usb4-port-number", 
                                                    Zero
                                                }
                                            }
                                        })
                                    }
                                }
                            }
                        }

                        Device (TBP3)
                        {
                            Name (_ADR, 0x00030000)  // _ADR: Address
                            Method (_S0W, 0, Serialized)  // _S0W: S0 Device Wake State
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS3 _S0W - RP SLOT ", ToHexString (SLOT)))
                                If ((\DTRT == Zero))
                                {
                                    ADBG ("[dTBT] DTBT DS3 S0W return 3")
                                    Return (0x03)
                                }
                                Else
                                {
                                    ADBG ("[dTBT] DTBT DS3 S0W return 4")
                                    Return (0x04)
                                }
                            }

                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS3 _PS0 - RP SLOT ", ToHexString (SLOT)))
                            }

                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS3 _PS3 - RP SLOT ", ToHexString (SLOT)))
                            }

                            If ((\_SB.PC00.RP09.TUID <= One))
                            {
                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                {
                                    ADBG (Concatenate ("[dTBT] dTBT DS3 _DSD - RP SLOT ", ToHexString (SLOT)))
                                    If ((\_SB.PC00.RP09.TUID == Zero))
                                    {
                                        Return (Package (0x02)
                                        {
                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                            Package (0x02)
                                            {
                                                Package (0x02)
                                                {
                                                    "usb4-host-interface", 
                                                    \_SB.PC00.RP09.PXSX.TBDH.NHI4
                                                }, 

                                                Package (0x02)
                                                {
                                                    "usb4-port-number", 
                                                    One
                                                }
                                            }
                                        })
                                    }
                                    Else
                                    {
                                        Return (Package (0x02)
                                        {
                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                            Package (0x02)
                                            {
                                                Package (0x02)
                                                {
                                                    "usb4-host-interface", 
                                                    \_SB.PC00.RP09.PXSX.TBDH.NHI5
                                                }, 

                                                Package (0x02)
                                                {
                                                    "usb4-port-number", 
                                                    One
                                                }
                                            }
                                        })
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        If (((DTFS == One) && (RPS0 == 0x0A)))
        {
            If ((RPT0 == 0x02))
            {
                Scope (\_SB.PC00.RP10.PXSX)
                {
                    Name (TURP, 0x0A)
                    Name (RPSN, One)
                    Device (TBDU)
                    {
                        Name (_ADR, 0x00020000)  // _ADR: Address
                        Device (XHCI)
                        {
                            Name (_ADR, Zero)  // _ADR: Address
                            Method (_S0W, 0, NotSerialized)  // _S0W: S0 Device Wake State
                            {
                                ADBG (Concatenate ("[dTBT] DTBT XHCI S0W - RP SLOT ", ToHexString (SLOT)))
                                Return (0x04)
                            }

                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                            {
                                ADBG (Concatenate ("[dTBT] DTBT XHCI PS0 - RP SLOT ", ToHexString (SLOT)))
                            }

                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                            {
                                ADBG (Concatenate ("[dTBT] DTBT XHCI PS3 - RP SLOT ", ToHexString (SLOT)))
                            }

                            Device (RHUB)
                            {
                                Name (_ADR, Zero)  // _ADR: Address
                                Method (_DSM, 4, Serialized)  // _DSM: Device-Specific Method
                                {
                                    ADBG ("[dTBT] XHCI RHUB _DSM")
                                    If ((Arg0 == ToUUID ("ce2ee385-00e6-48cb-9f05-2edb927c4899") /* USB Controller */))
                                    {
                                        If ((Arg2 == Zero))
                                        {
                                            ADBG ("[dTBT] XHCI RHUB F0")
                                            Return (Buffer (One)
                                            {
                                                 0x81                                             // .
                                            })
                                        }

                                        If ((Arg2 == 0x07))
                                        {
                                            ADBG ("[dTBT] XHCI RHUB F7")
                                            Return (Buffer (One)
                                            {
                                                 0x01                                             // .
                                            })
                                        }
                                    }

                                    Return (Buffer (One)
                                    {
                                         0x00                                             // .
                                    })
                                }

                                Method (_S0W, 0, NotSerialized)  // _S0W: S0 Device Wake State
                                {
                                    ADBG (Concatenate ("[dTBT] DTBT XHCI RHUB S0W return 4 - RP SLOT ", ToHexString (SLOT)))
                                    Return (0x04)
                                }

                                Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                                {
                                    ADBG (Concatenate ("[dTBT] DTBT XHCI RHUB PS0 - RP SLOT ", ToHexString (SLOT)))
                                }

                                Method (_PS2, 0, Serialized)  // _PS2: Power State 2
                                {
                                    ADBG (Concatenate ("[dTBT] DTBT XHCI RHUB PS2 - RP SLOT ", ToHexString (SLOT)))
                                }

                                Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                                {
                                    ADBG (Concatenate ("[dTBT] DTBT XHCI RHUB PS3 - RP SLOT ", ToHexString (SLOT)))
                                }

                                Method (SLMS, 1, Serialized)
                                {
                                    Local0 = (TURP << 0x02)
                                    Local0 |= One
                                    If ((((TP1D >> One) == Local0) && (Arg0 == TP1T)))
                                    {
                                        Return ((TP1D & One))
                                    }
                                    ElseIf ((((TP2D >> One) == Local0) && (Arg0 == TP2T)))
                                    {
                                        Return ((TP2D & One))
                                    }
                                    ElseIf ((((TP3D >> One) == Local0) && (Arg0 == TP3T)))
                                    {
                                        Return ((TP3D & One))
                                    }
                                    ElseIf ((((TP4D >> One) == Local0) && (Arg0 == TP4T)))
                                    {
                                        Return ((TP4D & One))
                                    }
                                    ElseIf ((((TP5D >> One) == Local0) && (Arg0 == TP5T)))
                                    {
                                        Return ((TP5D & One))
                                    }
                                    ElseIf ((((TP6D >> One) == Local0) && (Arg0 == TP6T)))
                                    {
                                        Return ((TP6D & One))
                                    }
                                    Else
                                    {
                                        Return (Zero)
                                    }
                                }

                                Method (FPCP, 2, Serialized)
                                {
                                    Local0 = (TURP << 0x02)
                                    Local0 |= One
                                    If ((((TP1D >> One) == Local0) && (Arg0 == TP1T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (One, Arg1, TP1C))
                                    }
                                    ElseIf ((((TP2D >> One) == Local0) && (Arg0 == TP2T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x02, Arg1, TP2C))
                                    }
                                    ElseIf ((((TP3D >> One) == Local0) && (Arg0 == TP3T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x03, Arg1, TP3C))
                                    }
                                    ElseIf ((((TP4D >> One) == Local0) && (Arg0 == TP4T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x04, Arg1, TP4C))
                                    }
                                    ElseIf ((((TP5D >> One) == Local0) && (Arg0 == TP5T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x05, Arg1, TP5C))
                                    }
                                    ElseIf ((((TP6D >> One) == Local0) && (Arg0 == TP6T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x06, Arg1, TP6C))
                                    }
                                    ElseIf ((Arg1 == One))
                                    {
                                        Return (TUPC (Zero, Zero))
                                    }
                                    Else
                                    {
                                        Return (TPLD (Zero, Zero))
                                    }
                                }

                                Method (FPSP, 1, Serialized)
                                {
                                    Local0 = (TURP << 0x02)
                                    Local0 |= One
                                    If ((((TP1D >> One) == Local0) && (Arg0 == TP1T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP2D >> One) == Local0) && (Arg0 == TP2T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP3D >> One) == Local0) && (Arg0 == TP3T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP4D >> One) == Local0) && (Arg0 == TP4T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP5D >> One) == Local0) && (Arg0 == TP5T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP6D >> One) == Local0) && (Arg0 == TP6T)))
                                    {
                                        Return (One)
                                    }
                                    Else
                                    {
                                        Return (Zero)
                                    }
                                }

                                Method (TPLD, 2, Serialized)
                                {
                                    Name (PCKG, Package (0x01)
                                    {
                                        Buffer (0x10){}
                                    })
                                    CreateField (DerefOf (PCKG [Zero]), Zero, 0x07, REV)
                                    REV = One
                                    CreateField (DerefOf (PCKG [Zero]), 0x40, One, VISI)
                                    VISI = Arg0
                                    CreateField (DerefOf (PCKG [Zero]), 0x57, 0x08, GPOS)
                                    GPOS = Arg1
                                    CreateField (DerefOf (PCKG [Zero]), 0x4A, 0x04, SHAP)
                                    SHAP = One
                                    CreateField (DerefOf (PCKG [Zero]), 0x20, 0x10, WID)
                                    WID = 0x08
                                    CreateField (DerefOf (PCKG [Zero]), 0x30, 0x10, HGT)
                                    HGT = 0x03
                                    Return (PCKG) /* \_SB_.PC00.RP10.PXSX.TBDU.XHCI.RHUB.TPLD.PCKG */
                                }

                                Method (TUPC, 2, Serialized)
                                {
                                    Name (PCKG, Package (0x04)
                                    {
                                        One, 
                                        Zero, 
                                        Zero, 
                                        Zero
                                    })
                                    PCKG [Zero] = Arg0
                                    PCKG [One] = Arg1
                                    If ((RPSN == One))
                                    {
                                        PCKG [0x02] = TP3C /* External reference */
                                    }
                                    ElseIf ((RPSN == 0x02))
                                    {
                                        PCKG [0x02] = TP5C /* External reference */
                                    }

                                    Return (PCKG) /* \_SB_.PC00.RP10.PXSX.TBDU.XHCI.RHUB.TUPC.PCKG */
                                }

                                If ((FPSP (One) == One))
                                {
                                    Device (HS01)
                                    {
                                        Name (_ADR, One)  // _ADR: Address
                                        Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                                        {
                                            If ((SLMS (One) == Zero))
                                            {
                                                Return (FPCP (One, One))
                                            }
                                            Else
                                            {
                                                Return (TUPC (Zero, Zero))
                                            }
                                        }

                                        Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                                        {
                                            If ((SLMS (One) == Zero))
                                            {
                                                Return (FPCP (One, 0x02))
                                            }
                                            Else
                                            {
                                                Return (TPLD (Zero, Zero))
                                            }
                                        }
                                    }
                                }

                                If ((FPSP (0x02) == One))
                                {
                                    Device (HS02)
                                    {
                                        Name (_ADR, 0x02)  // _ADR: Address
                                        Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                                        {
                                            If ((SLMS (0x02) == Zero))
                                            {
                                                Return (FPCP (0x02, One))
                                            }
                                            Else
                                            {
                                                Return (TUPC (Zero, Zero))
                                            }
                                        }

                                        Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                                        {
                                            If ((SLMS (0x02) == Zero))
                                            {
                                                Return (FPCP (0x02, 0x02))
                                            }
                                            Else
                                            {
                                                Return (TPLD (Zero, Zero))
                                            }
                                        }
                                    }
                                }

                                If ((FPSP (One) == One))
                                {
                                    Device (SS01)
                                    {
                                        Name (_ADR, 0x03)  // _ADR: Address
                                        Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                                        {
                                            Return (FPCP (One, One))
                                        }

                                        Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                                        {
                                            Return (FPCP (One, 0x02))
                                        }

                                        If (CondRefOf (\_SB.PC00.RP10.TUID))
                                        {
                                            ADBG (Concatenate ("[dTBT] DTBT SS01 - RP SLOT ", ToHexString (SLOT)))
                                            If ((\_SB.PC00.RP10.TUID <= One))
                                            {
                                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                                {
                                                    ADBG (Concatenate ("[dTBT] DTBT SS01 DSD - RP SLOT ", ToHexString (SLOT)))
                                                    If ((\_SB.PC00.RP10.TUID == Zero))
                                                    {
                                                        Return (Package (0x02)
                                                        {
                                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                                            Package (0x02)
                                                            {
                                                                Package (0x02)
                                                                {
                                                                    "usb4-host-interface", 
                                                                    \_SB.PC00.RP10.PXSX.TBDH.NHI4
                                                                }, 

                                                                Package (0x02)
                                                                {
                                                                    "usb4-port-number", 
                                                                    Zero
                                                                }
                                                            }
                                                        })
                                                    }
                                                    Else
                                                    {
                                                        Return (Package (0x02)
                                                        {
                                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                                            Package (0x02)
                                                            {
                                                                Package (0x02)
                                                                {
                                                                    "usb4-host-interface", 
                                                                    \_SB.PC00.RP10.PXSX.TBDH.NHI5
                                                                }, 

                                                                Package (0x02)
                                                                {
                                                                    "usb4-port-number", 
                                                                    Zero
                                                                }
                                                            }
                                                        })
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                If ((FPSP (0x02) == One))
                                {
                                    Device (SS02)
                                    {
                                        Name (_ADR, 0x04)  // _ADR: Address
                                        Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                                        {
                                            Return (FPCP (0x02, One))
                                        }

                                        Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                                        {
                                            Return (FPCP (0x02, 0x02))
                                        }

                                        If (CondRefOf (\_SB.PC00.RP10.TUID))
                                        {
                                            ADBG (Concatenate ("[dTBT] DTBT SS02 - RP SLOT ", ToHexString (SLOT)))
                                            If ((\_SB.PC00.RP10.TUID <= One))
                                            {
                                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                                {
                                                    ADBG (Concatenate ("[dTBT] DTBT SS02 DSD - RP SLOT ", ToHexString (SLOT)))
                                                    If ((\_SB.PC00.RP10.TUID == Zero))
                                                    {
                                                        Return (Package (0x02)
                                                        {
                                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                                            Package (0x02)
                                                            {
                                                                Package (0x02)
                                                                {
                                                                    "usb4-host-interface", 
                                                                    \_SB.PC00.RP10.PXSX.TBDH.NHI4
                                                                }, 

                                                                Package (0x02)
                                                                {
                                                                    "usb4-port-number", 
                                                                    One
                                                                }
                                                            }
                                                        })
                                                    }
                                                    Else
                                                    {
                                                        Return (Package (0x02)
                                                        {
                                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                                            Package (0x02)
                                                            {
                                                                Package (0x02)
                                                                {
                                                                    "usb4-host-interface", 
                                                                    \_SB.PC00.RP10.PXSX.TBDH.NHI5
                                                                }, 

                                                                Package (0x02)
                                                                {
                                                                    "usb4-port-number", 
                                                                    One
                                                                }
                                                            }
                                                        })
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            Method (_RMV, 0, NotSerialized)  // _RMV: Removal Status
                            {
                                Return (Zero)
                            }
                        }
                    }

                    If ((CondRefOf (\_SB.PC00.RP10.TUID) && CondRefOf (\DTRT)))
                    {
                        Device (TBDH)
                        {
                            Name (_ADR, Zero)  // _ADR: Address
                            If ((\_SB.PC00.RP10.TUID <= One))
                            {
                                If ((\_SB.PC00.RP10.TUID == Zero))
                                {
                                    Device (NHI4)
                                    {
                                        Name (_ADR, Zero)  // _ADR: Address
                                        If (CondRefOf (\DTRT))
                                        {
                                            Method (_S0W, 0, Serialized)  // _S0W: S0 Device Wake State
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _S0W - RP SLOT ", ToHexString (SLOT)))
                                                If ((\DTRT == Zero))
                                                {
                                                    ADBG ("[dTBT] DTBT NHI S0W return 3")
                                                    Return (0x03)
                                                }
                                                Else
                                                {
                                                    ADBG ("[dTBT] DTBT NHI S0W return 4")
                                                    Return (0x04)
                                                }
                                            }

                                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS0 - RP SLOT ", ToHexString (SLOT)))
                                            }

                                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS3 Start - RP SLOT", ToHexString (SLOT)))
                                                If (!(CondRefOf (\RPS0) && CondRefOf (\RPT0)))
                                                {
                                                    ADBG ("[dTBT] dTBT NHI _PS3 End due to RPS# or RPT# not present!")
                                                    Return (Zero)
                                                }

                                                If (CondRefOf (\_SB.PC00.RP10.WAKG))
                                                {
                                                    ADBG (Concatenate ("[dTBT] WAKG = ", ToHexString (\_SB.PC00.RP10.WAKG)))
                                                    If ((\_SB.PC00.RP10.WAKG != Zero))
                                                    {
                                                        ADBG ("[dTBT] Enable Wake")
                                                        \_SB.PC00.RP10.DTWK (Zero)
                                                    }
                                                }

                                                If (CondRefOf (\DTMB))
                                                {
                                                    \DTMB (0x48, Zero, \RPS0, \RPT0)
                                                }

                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS3 End - RP SLOT ", ToHexString (SLOT)))
                                            }

                                            Method (_RMV, 0, NotSerialized)  // _RMV: Removal Status
                                            {
                                                Return (Zero)
                                            }
                                        }
                                    }
                                }
                                Else
                                {
                                    Device (NHI5)
                                    {
                                        Name (_ADR, Zero)  // _ADR: Address
                                        If (CondRefOf (\DTRT))
                                        {
                                            Method (_S0W, 0, Serialized)  // _S0W: S0 Device Wake State
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _S0W - RP SLOT ", ToHexString (SLOT)))
                                                If ((\DTRT == Zero))
                                                {
                                                    ADBG ("[dTBT] DTBT NHI S0W return 3")
                                                    Return (0x03)
                                                }
                                                Else
                                                {
                                                    ADBG ("[dTBT] DTBT NHI S0W return 4")
                                                    Return (0x04)
                                                }
                                            }

                                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS0 - RP SLOT ", ToHexString (SLOT)))
                                            }

                                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS3 Start - RP SLOT", ToHexString (SLOT)))
                                                If (!(CondRefOf (\RPS1) && CondRefOf (\RPT1)))
                                                {
                                                    ADBG ("[dTBT] dTBT NHI _PS3 End due to RPS# or RPT# not present!")
                                                    Return (Zero)
                                                }

                                                If (CondRefOf (\_SB.PC00.RP10.WAKG))
                                                {
                                                    ADBG (Concatenate ("[dTBT] WAKG = ", ToHexString (\_SB.PC00.RP10.WAKG)))
                                                    If ((\_SB.PC00.RP10.WAKG != Zero))
                                                    {
                                                        ADBG ("[dTBT] Enable Wake")
                                                        \_SB.PC00.RP10.DTWK (Zero)
                                                    }
                                                }

                                                If (CondRefOf (\DTMB))
                                                {
                                                    \DTMB (0x48, Zero, \RPS1, \RPT1)
                                                }

                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS3 End - RP SLOT ", ToHexString (SLOT)))
                                            }

                                            Method (_RMV, 0, NotSerialized)  // _RMV: Removal Status
                                            {
                                                Return (Zero)
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        Device (TBP1)
                        {
                            Name (_ADR, 0x00010000)  // _ADR: Address
                            Method (_S0W, 0, Serialized)  // _S0W: S0 Device Wake State
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS1 _S0W - RP SLOT ", ToHexString (SLOT)))
                                If ((\DTRT == Zero))
                                {
                                    ADBG ("[dTBT] DTBT DS1 S0W return 3")
                                    Return (0x03)
                                }
                                Else
                                {
                                    ADBG ("[dTBT] DTBT DS1 S0W return 4")
                                    Return (0x04)
                                }
                            }

                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS1 _PS0 - RP SLOT ", ToHexString (SLOT)))
                            }

                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS1 _PS3 - RP SLOT ", ToHexString (SLOT)))
                            }

                            If ((\_SB.PC00.RP10.TUID <= One))
                            {
                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                {
                                    ADBG (Concatenate ("[dTBT] dTBT DS1 _DSD - RP SLOT ", ToHexString (SLOT)))
                                    If ((\_SB.PC00.RP10.TUID == Zero))
                                    {
                                        Return (Package (0x02)
                                        {
                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                            Package (0x02)
                                            {
                                                Package (0x02)
                                                {
                                                    "usb4-host-interface", 
                                                    \_SB.PC00.RP10.PXSX.TBDH.NHI4
                                                }, 

                                                Package (0x02)
                                                {
                                                    "usb4-port-number", 
                                                    Zero
                                                }
                                            }
                                        })
                                    }
                                    Else
                                    {
                                        Return (Package (0x02)
                                        {
                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                            Package (0x02)
                                            {
                                                Package (0x02)
                                                {
                                                    "usb4-host-interface", 
                                                    \_SB.PC00.RP10.PXSX.TBDH.NHI5
                                                }, 

                                                Package (0x02)
                                                {
                                                    "usb4-port-number", 
                                                    Zero
                                                }
                                            }
                                        })
                                    }
                                }
                            }
                        }

                        Device (TBP3)
                        {
                            Name (_ADR, 0x00030000)  // _ADR: Address
                            Method (_S0W, 0, Serialized)  // _S0W: S0 Device Wake State
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS3 _S0W - RP SLOT ", ToHexString (SLOT)))
                                If ((\DTRT == Zero))
                                {
                                    ADBG ("[dTBT] DTBT DS3 S0W return 3")
                                    Return (0x03)
                                }
                                Else
                                {
                                    ADBG ("[dTBT] DTBT DS3 S0W return 4")
                                    Return (0x04)
                                }
                            }

                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS3 _PS0 - RP SLOT ", ToHexString (SLOT)))
                            }

                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS3 _PS3 - RP SLOT ", ToHexString (SLOT)))
                            }

                            If ((\_SB.PC00.RP10.TUID <= One))
                            {
                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                {
                                    ADBG (Concatenate ("[dTBT] dTBT DS3 _DSD - RP SLOT ", ToHexString (SLOT)))
                                    If ((\_SB.PC00.RP10.TUID == Zero))
                                    {
                                        Return (Package (0x02)
                                        {
                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                            Package (0x02)
                                            {
                                                Package (0x02)
                                                {
                                                    "usb4-host-interface", 
                                                    \_SB.PC00.RP10.PXSX.TBDH.NHI4
                                                }, 

                                                Package (0x02)
                                                {
                                                    "usb4-port-number", 
                                                    One
                                                }
                                            }
                                        })
                                    }
                                    Else
                                    {
                                        Return (Package (0x02)
                                        {
                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                            Package (0x02)
                                            {
                                                Package (0x02)
                                                {
                                                    "usb4-host-interface", 
                                                    \_SB.PC00.RP10.PXSX.TBDH.NHI5
                                                }, 

                                                Package (0x02)
                                                {
                                                    "usb4-port-number", 
                                                    One
                                                }
                                            }
                                        })
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        If (((DTFS == One) && ((RPS0 == 0x04) || (RPS1 == 0x04))))
        {
            If ((RPT0 == 0x02))
            {
                Scope (\_SB.PC00.RP17.PXSX)
                {
                    Name (TURP, 0x11)
                    Name (RPSN, One)
                    Device (TBDU)
                    {
                        Name (_ADR, 0x00020000)  // _ADR: Address
                        Device (XHCI)
                        {
                            Name (_ADR, Zero)  // _ADR: Address
                            Method (_S0W, 0, NotSerialized)  // _S0W: S0 Device Wake State
                            {
                                ADBG (Concatenate ("[dTBT] DTBT XHCI S0W - RP SLOT ", ToHexString (SLOT)))
                                Return (0x04)
                            }

                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                            {
                                ADBG (Concatenate ("[dTBT] DTBT XHCI PS0 - RP SLOT ", ToHexString (SLOT)))
                            }

                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                            {
                                ADBG (Concatenate ("[dTBT] DTBT XHCI PS3 - RP SLOT ", ToHexString (SLOT)))
                            }

                            Device (RHUB)
                            {
                                Name (_ADR, Zero)  // _ADR: Address
                                Method (_DSM, 4, Serialized)  // _DSM: Device-Specific Method
                                {
                                    ADBG ("[dTBT] XHCI RHUB _DSM")
                                    If ((Arg0 == ToUUID ("ce2ee385-00e6-48cb-9f05-2edb927c4899") /* USB Controller */))
                                    {
                                        If ((Arg2 == Zero))
                                        {
                                            ADBG ("[dTBT] XHCI RHUB F0")
                                            Return (Buffer (One)
                                            {
                                                 0x81                                             // .
                                            })
                                        }

                                        If ((Arg2 == 0x07))
                                        {
                                            ADBG ("[dTBT] XHCI RHUB F7")
                                            Return (Buffer (One)
                                            {
                                                 0x01                                             // .
                                            })
                                        }
                                    }

                                    Return (Buffer (One)
                                    {
                                         0x00                                             // .
                                    })
                                }

                                Method (_S0W, 0, NotSerialized)  // _S0W: S0 Device Wake State
                                {
                                    ADBG (Concatenate ("[dTBT] DTBT XHCI RHUB S0W return 4 - RP SLOT ", ToHexString (SLOT)))
                                    Return (0x04)
                                }

                                Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                                {
                                    ADBG (Concatenate ("[dTBT] DTBT XHCI RHUB PS0 - RP SLOT ", ToHexString (SLOT)))
                                }

                                Method (_PS2, 0, Serialized)  // _PS2: Power State 2
                                {
                                    ADBG (Concatenate ("[dTBT] DTBT XHCI RHUB PS2 - RP SLOT ", ToHexString (SLOT)))
                                }

                                Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                                {
                                    ADBG (Concatenate ("[dTBT] DTBT XHCI RHUB PS3 - RP SLOT ", ToHexString (SLOT)))
                                }

                                Method (SLMS, 1, Serialized)
                                {
                                    Local0 = (TURP << 0x02)
                                    Local0 |= One
                                    If ((((TP1D >> One) == Local0) && (Arg0 == TP1T)))
                                    {
                                        Return ((TP1D & One))
                                    }
                                    ElseIf ((((TP2D >> One) == Local0) && (Arg0 == TP2T)))
                                    {
                                        Return ((TP2D & One))
                                    }
                                    ElseIf ((((TP3D >> One) == Local0) && (Arg0 == TP3T)))
                                    {
                                        Return ((TP3D & One))
                                    }
                                    ElseIf ((((TP4D >> One) == Local0) && (Arg0 == TP4T)))
                                    {
                                        Return ((TP4D & One))
                                    }
                                    ElseIf ((((TP5D >> One) == Local0) && (Arg0 == TP5T)))
                                    {
                                        Return ((TP5D & One))
                                    }
                                    ElseIf ((((TP6D >> One) == Local0) && (Arg0 == TP6T)))
                                    {
                                        Return ((TP6D & One))
                                    }
                                    Else
                                    {
                                        Return (Zero)
                                    }
                                }

                                Method (FPCP, 2, Serialized)
                                {
                                    Local0 = (TURP << 0x02)
                                    Local0 |= One
                                    If ((((TP1D >> One) == Local0) && (Arg0 == TP1T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (One, Arg1, TP1C))
                                    }
                                    ElseIf ((((TP2D >> One) == Local0) && (Arg0 == TP2T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x02, Arg1, TP2C))
                                    }
                                    ElseIf ((((TP3D >> One) == Local0) && (Arg0 == TP3T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x03, Arg1, TP3C))
                                    }
                                    ElseIf ((((TP4D >> One) == Local0) && (Arg0 == TP4T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x04, Arg1, TP4C))
                                    }
                                    ElseIf ((((TP5D >> One) == Local0) && (Arg0 == TP5T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x05, Arg1, TP5C))
                                    }
                                    ElseIf ((((TP6D >> One) == Local0) && (Arg0 == TP6T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x06, Arg1, TP6C))
                                    }
                                    ElseIf ((Arg1 == One))
                                    {
                                        Return (TUPC (Zero, Zero))
                                    }
                                    Else
                                    {
                                        Return (TPLD (Zero, Zero))
                                    }
                                }

                                Method (FPSP, 1, Serialized)
                                {
                                    Local0 = (TURP << 0x02)
                                    Local0 |= One
                                    If ((((TP1D >> One) == Local0) && (Arg0 == TP1T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP2D >> One) == Local0) && (Arg0 == TP2T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP3D >> One) == Local0) && (Arg0 == TP3T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP4D >> One) == Local0) && (Arg0 == TP4T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP5D >> One) == Local0) && (Arg0 == TP5T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP6D >> One) == Local0) && (Arg0 == TP6T)))
                                    {
                                        Return (One)
                                    }
                                    Else
                                    {
                                        Return (Zero)
                                    }
                                }

                                Method (TPLD, 2, Serialized)
                                {
                                    Name (PCKG, Package (0x01)
                                    {
                                        Buffer (0x10){}
                                    })
                                    CreateField (DerefOf (PCKG [Zero]), Zero, 0x07, REV)
                                    REV = One
                                    CreateField (DerefOf (PCKG [Zero]), 0x40, One, VISI)
                                    VISI = Arg0
                                    CreateField (DerefOf (PCKG [Zero]), 0x57, 0x08, GPOS)
                                    GPOS = Arg1
                                    CreateField (DerefOf (PCKG [Zero]), 0x4A, 0x04, SHAP)
                                    SHAP = One
                                    CreateField (DerefOf (PCKG [Zero]), 0x20, 0x10, WID)
                                    WID = 0x08
                                    CreateField (DerefOf (PCKG [Zero]), 0x30, 0x10, HGT)
                                    HGT = 0x03
                                    Return (PCKG) /* \_SB_.PC00.RP17.PXSX.TBDU.XHCI.RHUB.TPLD.PCKG */
                                }

                                Method (TUPC, 2, Serialized)
                                {
                                    Name (PCKG, Package (0x04)
                                    {
                                        One, 
                                        Zero, 
                                        Zero, 
                                        Zero
                                    })
                                    PCKG [Zero] = Arg0
                                    PCKG [One] = Arg1
                                    If ((RPSN == One))
                                    {
                                        PCKG [0x02] = TP3C /* External reference */
                                    }
                                    ElseIf ((RPSN == 0x02))
                                    {
                                        PCKG [0x02] = TP5C /* External reference */
                                    }

                                    Return (PCKG) /* \_SB_.PC00.RP17.PXSX.TBDU.XHCI.RHUB.TUPC.PCKG */
                                }

                                If ((FPSP (One) == One))
                                {
                                    Device (HS01)
                                    {
                                        Name (_ADR, One)  // _ADR: Address
                                        Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                                        {
                                            If ((SLMS (One) == Zero))
                                            {
                                                Return (FPCP (One, One))
                                            }
                                            Else
                                            {
                                                Return (TUPC (Zero, Zero))
                                            }
                                        }

                                        Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                                        {
                                            If ((SLMS (One) == Zero))
                                            {
                                                Return (FPCP (One, 0x02))
                                            }
                                            Else
                                            {
                                                Return (TPLD (Zero, Zero))
                                            }
                                        }
                                    }
                                }

                                If ((FPSP (0x02) == One))
                                {
                                    Device (HS02)
                                    {
                                        Name (_ADR, 0x02)  // _ADR: Address
                                        Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                                        {
                                            If ((SLMS (0x02) == Zero))
                                            {
                                                Return (FPCP (0x02, One))
                                            }
                                            Else
                                            {
                                                Return (TUPC (Zero, Zero))
                                            }
                                        }

                                        Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                                        {
                                            If ((SLMS (0x02) == Zero))
                                            {
                                                Return (FPCP (0x02, 0x02))
                                            }
                                            Else
                                            {
                                                Return (TPLD (Zero, Zero))
                                            }
                                        }
                                    }
                                }

                                If ((FPSP (One) == One))
                                {
                                    Device (SS01)
                                    {
                                        Name (_ADR, 0x03)  // _ADR: Address
                                        Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                                        {
                                            Return (FPCP (One, One))
                                        }

                                        Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                                        {
                                            Return (FPCP (One, 0x02))
                                        }

                                        If (CondRefOf (\_SB.PC00.RP17.TUID))
                                        {
                                            ADBG (Concatenate ("[dTBT] DTBT SS01 - RP SLOT ", ToHexString (SLOT)))
                                            If ((\_SB.PC00.RP17.TUID <= One))
                                            {
                                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                                {
                                                    ADBG (Concatenate ("[dTBT] DTBT SS01 DSD - RP SLOT ", ToHexString (SLOT)))
                                                    If ((\_SB.PC00.RP17.TUID == Zero))
                                                    {
                                                        Return (Package (0x02)
                                                        {
                                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                                            Package (0x02)
                                                            {
                                                                Package (0x02)
                                                                {
                                                                    "usb4-host-interface", 
                                                                    \_SB.PC00.RP17.PXSX.TBDH.NHI4
                                                                }, 

                                                                Package (0x02)
                                                                {
                                                                    "usb4-port-number", 
                                                                    Zero
                                                                }
                                                            }
                                                        })
                                                    }
                                                    Else
                                                    {
                                                        Return (Package (0x02)
                                                        {
                                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                                            Package (0x02)
                                                            {
                                                                Package (0x02)
                                                                {
                                                                    "usb4-host-interface", 
                                                                    \_SB.PC00.RP17.PXSX.TBDH.NHI5
                                                                }, 

                                                                Package (0x02)
                                                                {
                                                                    "usb4-port-number", 
                                                                    Zero
                                                                }
                                                            }
                                                        })
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                If ((FPSP (0x02) == One))
                                {
                                    Device (SS02)
                                    {
                                        Name (_ADR, 0x04)  // _ADR: Address
                                        Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                                        {
                                            Return (FPCP (0x02, One))
                                        }

                                        Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                                        {
                                            Return (FPCP (0x02, 0x02))
                                        }

                                        If (CondRefOf (\_SB.PC00.RP17.TUID))
                                        {
                                            ADBG (Concatenate ("[dTBT] DTBT SS02 - RP SLOT ", ToHexString (SLOT)))
                                            If ((\_SB.PC00.RP17.TUID <= One))
                                            {
                                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                                {
                                                    ADBG (Concatenate ("[dTBT] DTBT SS02 DSD - RP SLOT ", ToHexString (SLOT)))
                                                    If ((\_SB.PC00.RP17.TUID == Zero))
                                                    {
                                                        Return (Package (0x02)
                                                        {
                                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                                            Package (0x02)
                                                            {
                                                                Package (0x02)
                                                                {
                                                                    "usb4-host-interface", 
                                                                    \_SB.PC00.RP17.PXSX.TBDH.NHI4
                                                                }, 

                                                                Package (0x02)
                                                                {
                                                                    "usb4-port-number", 
                                                                    One
                                                                }
                                                            }
                                                        })
                                                    }
                                                    Else
                                                    {
                                                        Return (Package (0x02)
                                                        {
                                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                                            Package (0x02)
                                                            {
                                                                Package (0x02)
                                                                {
                                                                    "usb4-host-interface", 
                                                                    \_SB.PC00.RP17.PXSX.TBDH.NHI5
                                                                }, 

                                                                Package (0x02)
                                                                {
                                                                    "usb4-port-number", 
                                                                    One
                                                                }
                                                            }
                                                        })
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            Method (_RMV, 0, NotSerialized)  // _RMV: Removal Status
                            {
                                Return (Zero)
                            }
                        }
                    }

                    If ((CondRefOf (\_SB.PC00.RP17.TUID) && CondRefOf (\DTRT)))
                    {
                        Device (TBDH)
                        {
                            Name (_ADR, Zero)  // _ADR: Address
                            If ((\_SB.PC00.RP17.TUID <= One))
                            {
                                If ((\_SB.PC00.RP17.TUID == Zero))
                                {
                                    Device (NHI4)
                                    {
                                        Name (_ADR, Zero)  // _ADR: Address
                                        If (CondRefOf (\DTRT))
                                        {
                                            Method (_S0W, 0, Serialized)  // _S0W: S0 Device Wake State
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _S0W - RP SLOT ", ToHexString (SLOT)))
                                                If ((\DTRT == Zero))
                                                {
                                                    ADBG ("[dTBT] DTBT NHI S0W return 3")
                                                    Return (0x03)
                                                }
                                                Else
                                                {
                                                    ADBG ("[dTBT] DTBT NHI S0W return 4")
                                                    Return (0x04)
                                                }
                                            }

                                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS0 - RP SLOT ", ToHexString (SLOT)))
                                            }

                                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS3 Start - RP SLOT", ToHexString (SLOT)))
                                                If (!(CondRefOf (\RPS0) && CondRefOf (\RPT0)))
                                                {
                                                    ADBG ("[dTBT] dTBT NHI _PS3 End due to RPS# or RPT# not present!")
                                                    Return (Zero)
                                                }

                                                If (CondRefOf (\_SB.PC00.RP17.WAKG))
                                                {
                                                    ADBG (Concatenate ("[dTBT] WAKG = ", ToHexString (\_SB.PC00.RP17.WAKG)))
                                                    If ((\_SB.PC00.RP17.WAKG != Zero))
                                                    {
                                                        ADBG ("[dTBT] Enable Wake")
                                                        \_SB.PC00.RP17.DTWK (Zero)
                                                    }
                                                }

                                                If (CondRefOf (\DTMB))
                                                {
                                                    \DTMB (0x48, Zero, \RPS0, \RPT0)
                                                }

                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS3 End - RP SLOT ", ToHexString (SLOT)))
                                            }

                                            Method (_RMV, 0, NotSerialized)  // _RMV: Removal Status
                                            {
                                                Return (Zero)
                                            }
                                        }
                                    }
                                }
                                Else
                                {
                                    Device (NHI5)
                                    {
                                        Name (_ADR, Zero)  // _ADR: Address
                                        If (CondRefOf (\DTRT))
                                        {
                                            Method (_S0W, 0, Serialized)  // _S0W: S0 Device Wake State
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _S0W - RP SLOT ", ToHexString (SLOT)))
                                                If ((\DTRT == Zero))
                                                {
                                                    ADBG ("[dTBT] DTBT NHI S0W return 3")
                                                    Return (0x03)
                                                }
                                                Else
                                                {
                                                    ADBG ("[dTBT] DTBT NHI S0W return 4")
                                                    Return (0x04)
                                                }
                                            }

                                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS0 - RP SLOT ", ToHexString (SLOT)))
                                            }

                                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS3 Start - RP SLOT", ToHexString (SLOT)))
                                                If (!(CondRefOf (\RPS1) && CondRefOf (\RPT1)))
                                                {
                                                    ADBG ("[dTBT] dTBT NHI _PS3 End due to RPS# or RPT# not present!")
                                                    Return (Zero)
                                                }

                                                If (CondRefOf (\_SB.PC00.RP17.WAKG))
                                                {
                                                    ADBG (Concatenate ("[dTBT] WAKG = ", ToHexString (\_SB.PC00.RP17.WAKG)))
                                                    If ((\_SB.PC00.RP17.WAKG != Zero))
                                                    {
                                                        ADBG ("[dTBT] Enable Wake")
                                                        \_SB.PC00.RP17.DTWK (Zero)
                                                    }
                                                }

                                                If (CondRefOf (\DTMB))
                                                {
                                                    \DTMB (0x48, Zero, \RPS1, \RPT1)
                                                }

                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS3 End - RP SLOT ", ToHexString (SLOT)))
                                            }

                                            Method (_RMV, 0, NotSerialized)  // _RMV: Removal Status
                                            {
                                                Return (Zero)
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        Device (TBP1)
                        {
                            Name (_ADR, 0x00010000)  // _ADR: Address
                            Method (_S0W, 0, Serialized)  // _S0W: S0 Device Wake State
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS1 _S0W - RP SLOT ", ToHexString (SLOT)))
                                If ((\DTRT == Zero))
                                {
                                    ADBG ("[dTBT] DTBT DS1 S0W return 3")
                                    Return (0x03)
                                }
                                Else
                                {
                                    ADBG ("[dTBT] DTBT DS1 S0W return 4")
                                    Return (0x04)
                                }
                            }

                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS1 _PS0 - RP SLOT ", ToHexString (SLOT)))
                            }

                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS1 _PS3 - RP SLOT ", ToHexString (SLOT)))
                            }

                            If ((\_SB.PC00.RP17.TUID <= One))
                            {
                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                {
                                    ADBG (Concatenate ("[dTBT] dTBT DS1 _DSD - RP SLOT ", ToHexString (SLOT)))
                                    If ((\_SB.PC00.RP17.TUID == Zero))
                                    {
                                        Return (Package (0x02)
                                        {
                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                            Package (0x02)
                                            {
                                                Package (0x02)
                                                {
                                                    "usb4-host-interface", 
                                                    \_SB.PC00.RP17.PXSX.TBDH.NHI4
                                                }, 

                                                Package (0x02)
                                                {
                                                    "usb4-port-number", 
                                                    Zero
                                                }
                                            }
                                        })
                                    }
                                    Else
                                    {
                                        Return (Package (0x02)
                                        {
                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                            Package (0x02)
                                            {
                                                Package (0x02)
                                                {
                                                    "usb4-host-interface", 
                                                    \_SB.PC00.RP17.PXSX.TBDH.NHI5
                                                }, 

                                                Package (0x02)
                                                {
                                                    "usb4-port-number", 
                                                    Zero
                                                }
                                            }
                                        })
                                    }
                                }
                            }
                        }

                        Device (TBP3)
                        {
                            Name (_ADR, 0x00030000)  // _ADR: Address
                            Method (_S0W, 0, Serialized)  // _S0W: S0 Device Wake State
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS3 _S0W - RP SLOT ", ToHexString (SLOT)))
                                If ((\DTRT == Zero))
                                {
                                    ADBG ("[dTBT] DTBT DS3 S0W return 3")
                                    Return (0x03)
                                }
                                Else
                                {
                                    ADBG ("[dTBT] DTBT DS3 S0W return 4")
                                    Return (0x04)
                                }
                            }

                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS3 _PS0 - RP SLOT ", ToHexString (SLOT)))
                            }

                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS3 _PS3 - RP SLOT ", ToHexString (SLOT)))
                            }

                            If ((\_SB.PC00.RP17.TUID <= One))
                            {
                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                {
                                    ADBG (Concatenate ("[dTBT] dTBT DS3 _DSD - RP SLOT ", ToHexString (SLOT)))
                                    If ((\_SB.PC00.RP17.TUID == Zero))
                                    {
                                        Return (Package (0x02)
                                        {
                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                            Package (0x02)
                                            {
                                                Package (0x02)
                                                {
                                                    "usb4-host-interface", 
                                                    \_SB.PC00.RP17.PXSX.TBDH.NHI4
                                                }, 

                                                Package (0x02)
                                                {
                                                    "usb4-port-number", 
                                                    One
                                                }
                                            }
                                        })
                                    }
                                    Else
                                    {
                                        Return (Package (0x02)
                                        {
                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                            Package (0x02)
                                            {
                                                Package (0x02)
                                                {
                                                    "usb4-host-interface", 
                                                    \_SB.PC00.RP17.PXSX.TBDH.NHI5
                                                }, 

                                                Package (0x02)
                                                {
                                                    "usb4-port-number", 
                                                    One
                                                }
                                            }
                                        })
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        If (((DTFS == One) && ((RPS0 == 0x09) || (RPS1 == 0x09))))
        {
            If ((RPT1 == One))
            {
                Scope (\_SB.PC02.RP09.PXSX)
                {
                    Name (TURP, 0x09)
                    Name (RPSN, 0x02)
                    Device (TBDU)
                    {
                        Name (_ADR, 0x00020000)  // _ADR: Address
                        Device (XHCI)
                        {
                            Name (_ADR, Zero)  // _ADR: Address
                            Method (_S0W, 0, NotSerialized)  // _S0W: S0 Device Wake State
                            {
                                ADBG (Concatenate ("[dTBT] DTBT XHCI S0W - RP SLOT ", ToHexString (SLOT)))
                                Return (0x04)
                            }

                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                            {
                                ADBG (Concatenate ("[dTBT] DTBT XHCI PS0 - RP SLOT ", ToHexString (SLOT)))
                            }

                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                            {
                                ADBG (Concatenate ("[dTBT] DTBT XHCI PS3 - RP SLOT ", ToHexString (SLOT)))
                            }

                            Device (RHUB)
                            {
                                Name (_ADR, Zero)  // _ADR: Address
                                Method (_DSM, 4, Serialized)  // _DSM: Device-Specific Method
                                {
                                    ADBG ("[dTBT] XHCI RHUB _DSM")
                                    If ((Arg0 == ToUUID ("ce2ee385-00e6-48cb-9f05-2edb927c4899") /* USB Controller */))
                                    {
                                        If ((Arg2 == Zero))
                                        {
                                            ADBG ("[dTBT] XHCI RHUB F0")
                                            Return (Buffer (One)
                                            {
                                                 0x81                                             // .
                                            })
                                        }

                                        If ((Arg2 == 0x07))
                                        {
                                            ADBG ("[dTBT] XHCI RHUB F7")
                                            Return (Buffer (One)
                                            {
                                                 0x01                                             // .
                                            })
                                        }
                                    }

                                    Return (Buffer (One)
                                    {
                                         0x00                                             // .
                                    })
                                }

                                Method (_S0W, 0, NotSerialized)  // _S0W: S0 Device Wake State
                                {
                                    ADBG (Concatenate ("[dTBT] DTBT XHCI RHUB S0W return 4 - RP SLOT ", ToHexString (SLOT)))
                                    Return (0x04)
                                }

                                Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                                {
                                    ADBG (Concatenate ("[dTBT] DTBT XHCI RHUB PS0 - RP SLOT ", ToHexString (SLOT)))
                                }

                                Method (_PS2, 0, Serialized)  // _PS2: Power State 2
                                {
                                    ADBG (Concatenate ("[dTBT] DTBT XHCI RHUB PS2 - RP SLOT ", ToHexString (SLOT)))
                                }

                                Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                                {
                                    ADBG (Concatenate ("[dTBT] DTBT XHCI RHUB PS3 - RP SLOT ", ToHexString (SLOT)))
                                }

                                Method (SLMS, 1, Serialized)
                                {
                                    Local0 = (TURP << 0x02)
                                    Local0 |= One
                                    If ((((TP1D >> One) == Local0) && (Arg0 == TP1T)))
                                    {
                                        Return ((TP1D & One))
                                    }
                                    ElseIf ((((TP2D >> One) == Local0) && (Arg0 == TP2T)))
                                    {
                                        Return ((TP2D & One))
                                    }
                                    ElseIf ((((TP3D >> One) == Local0) && (Arg0 == TP3T)))
                                    {
                                        Return ((TP3D & One))
                                    }
                                    ElseIf ((((TP4D >> One) == Local0) && (Arg0 == TP4T)))
                                    {
                                        Return ((TP4D & One))
                                    }
                                    ElseIf ((((TP5D >> One) == Local0) && (Arg0 == TP5T)))
                                    {
                                        Return ((TP5D & One))
                                    }
                                    ElseIf ((((TP6D >> One) == Local0) && (Arg0 == TP6T)))
                                    {
                                        Return ((TP6D & One))
                                    }
                                    Else
                                    {
                                        Return (Zero)
                                    }
                                }

                                Method (FPCP, 2, Serialized)
                                {
                                    Local0 = (TURP << 0x02)
                                    Local0 |= One
                                    If ((((TP1D >> One) == Local0) && (Arg0 == TP1T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (One, Arg1, TP1C))
                                    }
                                    ElseIf ((((TP2D >> One) == Local0) && (Arg0 == TP2T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x02, Arg1, TP2C))
                                    }
                                    ElseIf ((((TP3D >> One) == Local0) && (Arg0 == TP3T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x03, Arg1, TP3C))
                                    }
                                    ElseIf ((((TP4D >> One) == Local0) && (Arg0 == TP4T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x04, Arg1, TP4C))
                                    }
                                    ElseIf ((((TP5D >> One) == Local0) && (Arg0 == TP5T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x05, Arg1, TP5C))
                                    }
                                    ElseIf ((((TP6D >> One) == Local0) && (Arg0 == TP6T)))
                                    {
                                        Return (\_SB.UBTC.RUCC (0x06, Arg1, TP6C))
                                    }
                                    ElseIf ((Arg1 == One))
                                    {
                                        Return (TUPC (Zero, Zero))
                                    }
                                    Else
                                    {
                                        Return (TPLD (Zero, Zero))
                                    }
                                }

                                Method (FPSP, 1, Serialized)
                                {
                                    Local0 = (TURP << 0x02)
                                    Local0 |= One
                                    If ((((TP1D >> One) == Local0) && (Arg0 == TP1T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP2D >> One) == Local0) && (Arg0 == TP2T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP3D >> One) == Local0) && (Arg0 == TP3T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP4D >> One) == Local0) && (Arg0 == TP4T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP5D >> One) == Local0) && (Arg0 == TP5T)))
                                    {
                                        Return (One)
                                    }
                                    ElseIf ((((TP6D >> One) == Local0) && (Arg0 == TP6T)))
                                    {
                                        Return (One)
                                    }
                                    Else
                                    {
                                        Return (Zero)
                                    }
                                }

                                Method (TPLD, 2, Serialized)
                                {
                                    Name (PCKG, Package (0x01)
                                    {
                                        Buffer (0x10){}
                                    })
                                    CreateField (DerefOf (PCKG [Zero]), Zero, 0x07, REV)
                                    REV = One
                                    CreateField (DerefOf (PCKG [Zero]), 0x40, One, VISI)
                                    VISI = Arg0
                                    CreateField (DerefOf (PCKG [Zero]), 0x57, 0x08, GPOS)
                                    GPOS = Arg1
                                    CreateField (DerefOf (PCKG [Zero]), 0x4A, 0x04, SHAP)
                                    SHAP = One
                                    CreateField (DerefOf (PCKG [Zero]), 0x20, 0x10, WID)
                                    WID = 0x08
                                    CreateField (DerefOf (PCKG [Zero]), 0x30, 0x10, HGT)
                                    HGT = 0x03
                                    Return (PCKG) /* \_SB_.PC02.RP09.PXSX.TBDU.XHCI.RHUB.TPLD.PCKG */
                                }

                                Method (TUPC, 2, Serialized)
                                {
                                    Name (PCKG, Package (0x04)
                                    {
                                        One, 
                                        Zero, 
                                        Zero, 
                                        Zero
                                    })
                                    PCKG [Zero] = Arg0
                                    PCKG [One] = Arg1
                                    If ((RPSN == One))
                                    {
                                        PCKG [0x02] = TP3C /* External reference */
                                    }
                                    ElseIf ((RPSN == 0x02))
                                    {
                                        PCKG [0x02] = TP5C /* External reference */
                                    }

                                    Return (PCKG) /* \_SB_.PC02.RP09.PXSX.TBDU.XHCI.RHUB.TUPC.PCKG */
                                }

                                If ((FPSP (One) == One))
                                {
                                    Device (HS01)
                                    {
                                        Name (_ADR, One)  // _ADR: Address
                                        Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                                        {
                                            If ((SLMS (One) == Zero))
                                            {
                                                Return (FPCP (One, One))
                                            }
                                            Else
                                            {
                                                Return (TUPC (Zero, Zero))
                                            }
                                        }

                                        Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                                        {
                                            If ((SLMS (One) == Zero))
                                            {
                                                Return (FPCP (One, 0x02))
                                            }
                                            Else
                                            {
                                                Return (TPLD (Zero, Zero))
                                            }
                                        }
                                    }
                                }

                                If ((FPSP (0x02) == One))
                                {
                                    Device (HS02)
                                    {
                                        Name (_ADR, 0x02)  // _ADR: Address
                                        Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                                        {
                                            If ((SLMS (0x02) == Zero))
                                            {
                                                Return (FPCP (0x02, One))
                                            }
                                            Else
                                            {
                                                Return (TUPC (Zero, Zero))
                                            }
                                        }

                                        Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                                        {
                                            If ((SLMS (0x02) == Zero))
                                            {
                                                Return (FPCP (0x02, 0x02))
                                            }
                                            Else
                                            {
                                                Return (TPLD (Zero, Zero))
                                            }
                                        }
                                    }
                                }

                                If ((FPSP (One) == One))
                                {
                                    Device (SS01)
                                    {
                                        Name (_ADR, 0x03)  // _ADR: Address
                                        Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                                        {
                                            Return (FPCP (One, One))
                                        }

                                        Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                                        {
                                            Return (FPCP (One, 0x02))
                                        }

                                        If (CondRefOf (\_SB.PC02.RP09.TUID))
                                        {
                                            ADBG (Concatenate ("[dTBT] DTBT SS01 - RP SLOT ", ToHexString (SLOT)))
                                            If ((\_SB.PC02.RP09.TUID <= One))
                                            {
                                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                                {
                                                    ADBG (Concatenate ("[dTBT] DTBT SS01 DSD - RP SLOT ", ToHexString (SLOT)))
                                                    If ((\_SB.PC02.RP09.TUID == Zero))
                                                    {
                                                        Return (Package (0x02)
                                                        {
                                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                                            Package (0x02)
                                                            {
                                                                Package (0x02)
                                                                {
                                                                    "usb4-host-interface", 
                                                                    \_SB.PC02.RP09.PXSX.TBDH.NHI4
                                                                }, 

                                                                Package (0x02)
                                                                {
                                                                    "usb4-port-number", 
                                                                    Zero
                                                                }
                                                            }
                                                        })
                                                    }
                                                    Else
                                                    {
                                                        Return (Package (0x02)
                                                        {
                                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                                            Package (0x02)
                                                            {
                                                                Package (0x02)
                                                                {
                                                                    "usb4-host-interface", 
                                                                    \_SB.PC02.RP09.PXSX.TBDH.NHI5
                                                                }, 

                                                                Package (0x02)
                                                                {
                                                                    "usb4-port-number", 
                                                                    Zero
                                                                }
                                                            }
                                                        })
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                If ((FPSP (0x02) == One))
                                {
                                    Device (SS02)
                                    {
                                        Name (_ADR, 0x04)  // _ADR: Address
                                        Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                                        {
                                            Return (FPCP (0x02, One))
                                        }

                                        Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                                        {
                                            Return (FPCP (0x02, 0x02))
                                        }

                                        If (CondRefOf (\_SB.PC02.RP09.TUID))
                                        {
                                            ADBG (Concatenate ("[dTBT] DTBT SS02 - RP SLOT ", ToHexString (SLOT)))
                                            If ((\_SB.PC02.RP09.TUID <= One))
                                            {
                                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                                {
                                                    ADBG (Concatenate ("[dTBT] DTBT SS02 DSD - RP SLOT ", ToHexString (SLOT)))
                                                    If ((\_SB.PC02.RP09.TUID == Zero))
                                                    {
                                                        Return (Package (0x02)
                                                        {
                                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                                            Package (0x02)
                                                            {
                                                                Package (0x02)
                                                                {
                                                                    "usb4-host-interface", 
                                                                    \_SB.PC02.RP09.PXSX.TBDH.NHI4
                                                                }, 

                                                                Package (0x02)
                                                                {
                                                                    "usb4-port-number", 
                                                                    One
                                                                }
                                                            }
                                                        })
                                                    }
                                                    Else
                                                    {
                                                        Return (Package (0x02)
                                                        {
                                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                                            Package (0x02)
                                                            {
                                                                Package (0x02)
                                                                {
                                                                    "usb4-host-interface", 
                                                                    \_SB.PC02.RP09.PXSX.TBDH.NHI5
                                                                }, 

                                                                Package (0x02)
                                                                {
                                                                    "usb4-port-number", 
                                                                    One
                                                                }
                                                            }
                                                        })
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            Method (_RMV, 0, NotSerialized)  // _RMV: Removal Status
                            {
                                Return (Zero)
                            }
                        }
                    }

                    If ((CondRefOf (\_SB.PC02.RP09.TUID) && CondRefOf (\DTRT)))
                    {
                        Device (TBDH)
                        {
                            Name (_ADR, Zero)  // _ADR: Address
                            If ((\_SB.PC02.RP09.TUID <= One))
                            {
                                If ((\_SB.PC02.RP09.TUID == Zero))
                                {
                                    Device (NHI4)
                                    {
                                        Name (_ADR, Zero)  // _ADR: Address
                                        If (CondRefOf (\DTRT))
                                        {
                                            Method (_S0W, 0, Serialized)  // _S0W: S0 Device Wake State
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _S0W - RP SLOT ", ToHexString (SLOT)))
                                                If ((\DTRT == Zero))
                                                {
                                                    ADBG ("[dTBT] DTBT NHI S0W return 3")
                                                    Return (0x03)
                                                }
                                                Else
                                                {
                                                    ADBG ("[dTBT] DTBT NHI S0W return 4")
                                                    Return (0x04)
                                                }
                                            }

                                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS0 - RP SLOT ", ToHexString (SLOT)))
                                            }

                                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS3 Start - RP SLOT", ToHexString (SLOT)))
                                                If (!(CondRefOf (\RPS0) && CondRefOf (\RPT0)))
                                                {
                                                    ADBG ("[dTBT] dTBT NHI _PS3 End due to RPS# or RPT# not present!")
                                                    Return (Zero)
                                                }

                                                If (CondRefOf (\_SB.PC02.RP09.WAKG))
                                                {
                                                    ADBG (Concatenate ("[dTBT] WAKG = ", ToHexString (\_SB.PC02.RP09.WAKG)))
                                                    If ((\_SB.PC02.RP09.WAKG != Zero))
                                                    {
                                                        ADBG ("[dTBT] Enable Wake")
                                                        \_SB.PC02.RP09.DTWK (Zero)
                                                    }
                                                }

                                                If (CondRefOf (\DTMB))
                                                {
                                                    \DTMB (0x48, Zero, \RPS0, \RPT0)
                                                }

                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS3 End - RP SLOT ", ToHexString (SLOT)))
                                            }

                                            Method (_RMV, 0, NotSerialized)  // _RMV: Removal Status
                                            {
                                                Return (Zero)
                                            }
                                        }
                                    }
                                }
                                Else
                                {
                                    Device (NHI5)
                                    {
                                        Name (_ADR, Zero)  // _ADR: Address
                                        If (CondRefOf (\DTRT))
                                        {
                                            Method (_S0W, 0, Serialized)  // _S0W: S0 Device Wake State
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _S0W - RP SLOT ", ToHexString (SLOT)))
                                                If ((\DTRT == Zero))
                                                {
                                                    ADBG ("[dTBT] DTBT NHI S0W return 3")
                                                    Return (0x03)
                                                }
                                                Else
                                                {
                                                    ADBG ("[dTBT] DTBT NHI S0W return 4")
                                                    Return (0x04)
                                                }
                                            }

                                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS0 - RP SLOT ", ToHexString (SLOT)))
                                            }

                                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                                            {
                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS3 Start - RP SLOT", ToHexString (SLOT)))
                                                If (!(CondRefOf (\RPS1) && CondRefOf (\RPT1)))
                                                {
                                                    ADBG ("[dTBT] dTBT NHI _PS3 End due to RPS# or RPT# not present!")
                                                    Return (Zero)
                                                }

                                                If (CondRefOf (\_SB.PC02.RP09.WAKG))
                                                {
                                                    ADBG (Concatenate ("[dTBT] WAKG = ", ToHexString (\_SB.PC02.RP09.WAKG)))
                                                    If ((\_SB.PC02.RP09.WAKG != Zero))
                                                    {
                                                        ADBG ("[dTBT] Enable Wake")
                                                        \_SB.PC02.RP09.DTWK (Zero)
                                                    }
                                                }

                                                If (CondRefOf (\DTMB))
                                                {
                                                    \DTMB (0x48, Zero, \RPS1, \RPT1)
                                                }

                                                ADBG (Concatenate ("[dTBT] dTBT NHI _PS3 End - RP SLOT ", ToHexString (SLOT)))
                                            }

                                            Method (_RMV, 0, NotSerialized)  // _RMV: Removal Status
                                            {
                                                Return (Zero)
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        Device (TBP1)
                        {
                            Name (_ADR, 0x00010000)  // _ADR: Address
                            Method (_S0W, 0, Serialized)  // _S0W: S0 Device Wake State
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS1 _S0W - RP SLOT ", ToHexString (SLOT)))
                                If ((\DTRT == Zero))
                                {
                                    ADBG ("[dTBT] DTBT DS1 S0W return 3")
                                    Return (0x03)
                                }
                                Else
                                {
                                    ADBG ("[dTBT] DTBT DS1 S0W return 4")
                                    Return (0x04)
                                }
                            }

                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS1 _PS0 - RP SLOT ", ToHexString (SLOT)))
                            }

                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS1 _PS3 - RP SLOT ", ToHexString (SLOT)))
                            }

                            If ((\_SB.PC02.RP09.TUID <= One))
                            {
                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                {
                                    ADBG (Concatenate ("[dTBT] dTBT DS1 _DSD - RP SLOT ", ToHexString (SLOT)))
                                    If ((\_SB.PC02.RP09.TUID == Zero))
                                    {
                                        Return (Package (0x02)
                                        {
                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                            Package (0x02)
                                            {
                                                Package (0x02)
                                                {
                                                    "usb4-host-interface", 
                                                    \_SB.PC02.RP09.PXSX.TBDH.NHI4
                                                }, 

                                                Package (0x02)
                                                {
                                                    "usb4-port-number", 
                                                    Zero
                                                }
                                            }
                                        })
                                    }
                                    Else
                                    {
                                        Return (Package (0x02)
                                        {
                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                            Package (0x02)
                                            {
                                                Package (0x02)
                                                {
                                                    "usb4-host-interface", 
                                                    \_SB.PC02.RP09.PXSX.TBDH.NHI5
                                                }, 

                                                Package (0x02)
                                                {
                                                    "usb4-port-number", 
                                                    Zero
                                                }
                                            }
                                        })
                                    }
                                }
                            }
                        }

                        Device (TBP3)
                        {
                            Name (_ADR, 0x00030000)  // _ADR: Address
                            Method (_S0W, 0, Serialized)  // _S0W: S0 Device Wake State
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS3 _S0W - RP SLOT ", ToHexString (SLOT)))
                                If ((\DTRT == Zero))
                                {
                                    ADBG ("[dTBT] DTBT DS3 S0W return 3")
                                    Return (0x03)
                                }
                                Else
                                {
                                    ADBG ("[dTBT] DTBT DS3 S0W return 4")
                                    Return (0x04)
                                }
                            }

                            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS3 _PS0 - RP SLOT ", ToHexString (SLOT)))
                            }

                            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                            {
                                ADBG (Concatenate ("[dTBT] dTBT DS3 _PS3 - RP SLOT ", ToHexString (SLOT)))
                            }

                            If ((\_SB.PC02.RP09.TUID <= One))
                            {
                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                {
                                    ADBG (Concatenate ("[dTBT] dTBT DS3 _DSD - RP SLOT ", ToHexString (SLOT)))
                                    If ((\_SB.PC02.RP09.TUID == Zero))
                                    {
                                        Return (Package (0x02)
                                        {
                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                            Package (0x02)
                                            {
                                                Package (0x02)
                                                {
                                                    "usb4-host-interface", 
                                                    \_SB.PC02.RP09.PXSX.TBDH.NHI4
                                                }, 

                                                Package (0x02)
                                                {
                                                    "usb4-port-number", 
                                                    One
                                                }
                                            }
                                        })
                                    }
                                    Else
                                    {
                                        Return (Package (0x02)
                                        {
                                            ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                            Package (0x02)
                                            {
                                                Package (0x02)
                                                {
                                                    "usb4-host-interface", 
                                                    \_SB.PC02.RP09.PXSX.TBDH.NHI5
                                                }, 

                                                Package (0x02)
                                                {
                                                    "usb4-port-number", 
                                                    One
                                                }
                                            }
                                        })
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

