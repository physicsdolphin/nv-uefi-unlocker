/*
 * Intel ACPI Component Architecture
 * AML/ASL+ Disassembler version 20251212 (64-bit version)
 * Copyright (c) 2000 - 2025 Intel Corporation
 * 
 * Disassembling to symbolic ASL+ operators
 *
 * Disassembly of ssdt-0d.aml
 *
 * Original Table Header:
 *     Signature        "SSDT"
 *     Length           0x00009CEA (40170)
 *     Revision         0x02
 *     Checksum         0x72
 *     OEM ID           "LENOVO"
 *     OEM Table ID     "TcssSsdt"
 *     OEM Revision     0x00001000 (4096)
 *     Compiler ID      "INTL"
 *     Compiler Version 0x20210930 (539035952)
 */
DefinitionBlock ("", "SSDT", 2, "LENOVO", "TcssSsdt", 0x00001000)
{
    External (_SB_.PC00, DeviceObj)
    External (_SB_.PC00.GFX0, DeviceObj)
    External (_SB_.PC00.GFX0.GFPS, MethodObj)    // 0 Arguments
    External (_SB_.PC00.GMHB, MethodObj)    // 0 Arguments
    External (_SB_.PC00.GPCB, MethodObj)    // 0 Arguments
    External (_SB_.PC00.PCIC, MethodObj)    // 1 Arguments
    External (_SB_.PC00.PCID, MethodObj)    // 4 Arguments
    External (_SB_.PC00.TDM0.SCME, UnknownObj)
    External (_SB_.PC00.TDM1.SCME, UnknownObj)
    External (_SB_.PC00.TXHC.CTEM, IntObj)
    External (ADBG, MethodObj)    // 1 Arguments
    External (CCPD, MethodObj)    // 2 Arguments
    External (CSFR, UnknownObj)
    External (DME0, UnknownObj)
    External (DME1, UnknownObj)
    External (ECR1, UnknownObj)
    External (GP1E, IntObj)
    External (GPRW, MethodObj)    // 2 Arguments
    External (IMRY, UnknownObj)
    External (IOMB, UnknownObj)
    External (ITM0, UnknownObj)
    External (ITM1, UnknownObj)
    External (LTE0, UnknownObj)
    External (LTE1, UnknownObj)
    External (LTE2, UnknownObj)
    External (LTE3, UnknownObj)
    External (OTHC, UnknownObj)
    External (P2PS, UnknownObj)
    External (PCIC, MethodObj)    // 1 Arguments
    External (PCID, MethodObj)    // 4 Arguments
    External (PICM, UnknownObj)
    External (PNS0, UnknownObj)
    External (PNS1, UnknownObj)
    External (PNS2, UnknownObj)
    External (PNS3, UnknownObj)
    External (PSL0, UnknownObj)
    External (PSL1, UnknownObj)
    External (PSL2, UnknownObj)
    External (PSL3, UnknownObj)
    External (REGO, UnknownObj)
    External (RPCC, UnknownObj)
    External (RTBM, UnknownObj)
    External (TCDS, UnknownObj)
    External (TCIR, UnknownObj)
    External (TCIT, UnknownObj)
    External (TDCE, UnknownObj)
    External (THCE, UnknownObj)
    External (TIVS, UnknownObj)
    External (TPA0, UnknownObj)
    External (TPA1, UnknownObj)
    External (TPA2, UnknownObj)
    External (TPA3, UnknownObj)
    External (TRE0, UnknownObj)
    External (TRE1, UnknownObj)
    External (TRE2, UnknownObj)
    External (TRE3, UnknownObj)
    External (TRTD, UnknownObj)
    External (U4CM, IntObj)
    External (XDAT, MethodObj)    // 0 Arguments

    Name (ITNB, 0x77EFB000)
    Name (ITNL, 0x0014)
    OperationRegion (ITNV, SystemMemory, ITNB, ITNL)
    Field (ITNV, AnyAcc, Lock, Preserve)
    {
        ITSP,   8, 
        IDM0,   8, 
        IDM1,   8, 
        ITCP,   8, 
        IT0E,   8, 
        IT1E,   8, 
        IT2E,   8, 
        IT3E,   8, 
        IORB,   8, 
        ITRT,   8, 
        ITRE,   16, 
        ITIM,   8, 
        ITFP,   16, 
        ITCT,   16, 
        ITVD,   8, 
        PLAT,   16
    }

    Scope (\_SB)
    {
        Name (C2PW, Zero)
        Method (C2PM, 4, Serialized)
        {
            Switch (ToInteger (Arg3))
            {
                Case (Zero)
                {
                    Local0 = "Tcss iTbt Pcie 0 RP 0"
                }
                Case (One)
                {
                    Local0 = "Tcss iTbt Pcie 0 RP 1"
                }
                Case (0x02)
                {
                    Local0 = "Tcss iTbt Pcie 0 RP 2"
                }
                Case (0x03)
                {
                    Local0 = "Tcss iTbt Pcie 0 RP 3"
                }
                Case (0x04)
                {
                    Local0 = "Tcss xHci"
                }
                Case (0x05)
                {
                    Local0 = "Tcss xDci"
                }
                Case (0x06)
                {
                    Local0 = "Tcss Dma 0"
                }
                Case (0x07)
                {
                    Local0 = "Tcss Dma 1"
                }
                Case (0x08)
                {
                    Local0 = "Tcss iTbt Pcie 1 RP 0"
                }
                Case (0x09)
                {
                    Local0 = "Tcss iTbt Pcie 1 RP 1"
                }
                Case (0x0A)
                {
                    Local0 = "Tcss iTbt Pcie 1 RP 2"
                }
                Case (0x0B)
                {
                    Local0 = "Tcss iTbt Pcie 1 RP 3"
                }
                Default
                {
                    Return (Zero)
                }

            }

            Local1 = Zero
            Local1 = (One << ToInteger (Arg3))
            ADBG (Concatenate ("Local1 = ", ToHexString (Local1)))
            ADBG (Concatenate ("Before CPU to PCH Wake capability configuration Start from ", Local0))
            ADBG (Concatenate (Local0, Concatenate (" Cpu To Pch Wake Value = ", ToHexString (C2PW))))
            ADBG (Concatenate ("C2PM ", Local0))
            If ((Arg0 && Arg1))
            {
                If (((C2PW & Local1) == Zero))
                {
                    C2PW |= Local1
                }

                ADBG (Concatenate (Local0, " Sx EnWak"))
            }
            ElseIf ((Arg0 || Arg2))
            {
                If (((C2PW & Local1) == Zero))
                {
                    C2PW |= Local1
                }

                ADBG (Concatenate (Local0, " D3 En Wak"))
            }
            Else
            {
                If (((C2PW & Local1) != Zero))
                {
                    C2PW &= ~Local1
                }

                ADBG (Concatenate (Local0, " DisWak"))
            }

            ADBG (Concatenate ("After CPU to PCH Wake capability configuration End from ", Local0))
            ADBG (Concatenate (Local0, Concatenate (" Cpu To Pch Wake Value = ", ToHexString (C2PW))))
            Local0 = Zero
            Return (Local0)
        }

        Method (TCWK, 1, NotSerialized)
        {
            If (((Arg0 == 0x03) || (Arg0 == 0x04)))
            {
                If ((\_SB.PC00.TRP0.VDID != 0xFFFFFFFF))
                {
                    Notify (\_SB.PC00.TRP0, Zero) // Bus Check
                }

                If ((\_SB.PC00.TRP1.VDID != 0xFFFFFFFF))
                {
                    Notify (\_SB.PC00.TRP1, Zero) // Bus Check
                }

                If ((\_SB.PC00.TRP2.VDID != 0xFFFFFFFF))
                {
                    Notify (\_SB.PC00.TRP2, Zero) // Bus Check
                }

                If ((\_SB.PC00.TRP3.VDID != 0xFFFFFFFF))
                {
                    Notify (\_SB.PC00.TRP3, Zero) // Bus Check
                }
            }
        }
    }

    Scope (\_SB.PC00)
    {
        OperationRegion (MBAR, SystemMemory, (\_SB.PC00.GMHB () + REGO), 0x1000)
        Field (MBAR, ByteAcc, NoLock, Preserve)
        {
            RBAR,   64
        }

        OperationRegion (PBAR, SystemMemory, (\_SB.PC00.GMHB () + 0x5DA0), 0x08)
        Field (PBAR, DWordAcc, NoLock, Preserve)
        {
            PMBD,   32, 
            PMBC,   8, 
            PSCM,   8, 
                ,   15, 
            PMBR,   1
        }

        Method (PMBY, 0, NotSerialized)
        {
            Local0 = Zero
            While ((PMBR && (Local0 < 0x03E8)))
            {
                Local0++
                Stall (One)
            }

            If ((Local0 == 0x03E8))
            {
                Return (0xFF)
            }

            Return (Zero)
        }

        OperationRegion (IOMR, SystemMemory, IOMB, 0x0100)
        Field (IOMR, DWordAcc, NoLock, Preserve)
        {
            Offset (0x40), 
                ,   15, 
            TD3C,   1, 
            TACK,   1, 
            DPOF,   1, 
                ,   2, 
            Offset (0x44), 
            DPHD,   1, 
            Offset (0x48), 
                ,   12, 
            INDP,   1, 
            Offset (0x70), 
            IMCD,   32, 
            IMDA,   32
        }

        Name (IGFG, Zero)
        Method (IMNG, 1, Serialized)
        {
            Switch (ToInteger (Arg0))
            {
                Case (0x03)
                {
                    If ((INDP == One))
                    {
                        IGFG = One
                    }

                    If ((DPHD == One))
                    {
                        DPHD = Zero
                    }
                }
                Case (0x04)
                {
                    If ((INDP == One))
                    {
                        IGFG = Zero
                    }

                    If ((DPHD == One))
                    {
                        DPHD = Zero
                    }
                }
                Case (0x71)
                {
                    Local0 = 0x0F
                    If (CondRefOf (\_SB.PC00.GFX0.GFPS))
                    {
                        Local0 = \_SB.PC00.GFX0.GFPS ()
                    }

                    If ((IGFG == Zero))
                    {
                        If (((INDP == One) && (Local0 == 0x03)))
                        {
                            ADBG (" Notify _SB.PC00.GFX0 ")
                            Notify (\_SB.PC00.GFX0, Zero) // Bus Check
                        }
                    }
                }

            }
        }

        Method (IMMD, 1, NotSerialized)
        {
            IMDA = Arg0
            ADBG (Concatenate ("IMMD :: ", ToHexString (IMDA)))
            Return (Zero)
        }

        Method (IMMC, 5, Serialized)
        {
            Name (OPTS, Buffer (0x04)
            {
                 0x00, 0x00, 0x00, 0x00                           // ....
            })
            CreateByteField (OPTS, Zero, CMST)
            CreateByteField (OPTS, One, RTB1)
            IMMD (Arg4)
            Local1 = Arg3
            Local1 <<= 0x08
            Local1 += Arg2
            Local1 <<= 0x08
            Local1 += Arg0
            IMCD = Local1
            ADBG (Concatenate ("Set IMMC Command ", ToHexString (IMCD)))
            IMCD |= 0x80000000
            ADBG (Concatenate ("Set IMMC Run Bit ", ToHexString (IMCD)))
            Local0 = Arg1
            ADBG (Concatenate ("IMMC Delay ", ToHexString (Local0)))
            While ((((IMCD & 0x80000000) != Zero) && (Local0 != 
                Zero)))
            {
                Sleep (One)
                Local0--
            }

            Switch (ToInteger (Arg0))
            {
                Case (0x02)
                {
                    ADBG ("GEM_SB_TRAN_CMD")
                    Return (Zero)
                }
                Case (0x03)
                {
                    ADBG ("IOM_BIOS_MBX_GET_HPD_COUNT")
                    If (((IMCD & 0x80000000) != Zero))
                    {
                        ADBG ("Timeout!")
                        ADBG (Concatenate ("IOM MB CMD Reg ", ToHexString (IMCD)))
                        ADBG (Concatenate ("IOM MB DATA Reg ", ToHexString (IMDA)))
                        CMST = One
                        Return (OPTS) /* \_SB_.PC00.IMMC.OPTS */
                    }
                    ElseIf (((IMCD & 0xFF) != Zero))
                    {
                        ADBG ("Completion Code Fail")
                        ADBG (Concatenate ("IOM MB CMD Reg ", ToHexString (IMCD)))
                        CMST = One
                    }
                    Else
                    {
                        ADBG ("Passed")
                        ADBG (Concatenate ("IOM MB CMD Reg :: ", ToHexString (IMCD)))
                        ADBG (Concatenate ("IOM MB DATA Reg :: ", ToHexString (IMDA)))
                        ADBG (Concatenate ("Arg2 is Port Number :: ", ToHexString (Arg2)))
                        CMST = Zero
                        RTB1 = IMDA /* \_SB_.PC00.IMDA */
                    }

                    Return (OPTS) /* \_SB_.PC00.IMMC.OPTS */
                }
                Case (0x06)
                {
                    ADBG ("BIOS_MBX_DEC_HPD_COUNT")
                    RTB1 = Zero
                    If (((IMCD & 0x80000000) != Zero))
                    {
                        ADBG ("Timeout!")
                        CMST = One
                    }
                    ElseIf (((IMCD & 0xFF) != Zero))
                    {
                        ADBG ("Completion Code Fail")
                        ADBG (Concatenate ("IOM MB CMD Reg ", ToHexString (IMCD)))
                        CMST = One
                        RTB1 = IMDA /* \_SB_.PC00.IMDA */
                    }
                    Else
                    {
                        ADBG ("Passed")
                        CMST = Zero
                    }

                    Return (OPTS) /* \_SB_.PC00.IMMC.OPTS */
                }
                Default
                {
                    CMST = One
                    Return (OPTS) /* \_SB_.PC00.IMMC.OPTS */
                }

            }
        }

        Name (CTP0, Zero)
        Name (CTP1, Zero)
        Method (TG0N, 0, NotSerialized)
        {
            ADBG ("TG0N Start")
            ADBG (Concatenate ("DMA0 VDID -", ToHexString (\_SB.PC00.TDM0.VDID)))
            ADBG (Concatenate ("DMA0 PMST -", ToHexString (\_SB.PC00.TDM0.PMST)))
            ADBG (Concatenate ("DMA0 PMEE -", ToHexString (\_SB.PC00.TDM0.PMEE)))
            ADBG (Concatenate ("DMA0 PMES -", ToHexString (\_SB.PC00.TDM0.PMES)))
            ADBG (Concatenate ("DMA0 STAT -", ToHexString (\_SB.PC00.TDM0.STAT)))
            If ((\_SB.PC00.TDM0.VDID != 0xFFFFFFFF))
            {
                If ((\_SB.PC00.TDM0.STAT == Zero))
                {
                    \_SB.PC00.TDM0.D3CX ()
                    ADBG ("Let\'s bring TBT RPs out of D3Cold")
                    If ((\_SB.PC00.TRP0.VDID != 0xFFFFFFFF))
                    {
                        \_SB.PC00.TRP0.D3CX ()
                    }

                    If ((\_SB.PC00.TRP1.VDID != 0xFFFFFFFF))
                    {
                        \_SB.PC00.TRP1.D3CX ()
                    }

                    If ((\_SB.PC00.TDM0.ALCT == One))
                    {
                        If ((CTP0 == One))
                        {
                            \_SB.PC00.TDM0.CNTP ()
                            \_SB.PC00.TDM0.WACT = One
                            CTP0 = Zero
                        }

                        \_SB.PC00.TDM0.ALCT = Zero
                    }
                }
                Else
                {
                    ADBG ("Drop TG0N due to it is already exit D3 cold")
                }

                Sleep (ITRE)
            }

            ADBG ("TG0N End")
        }

        Method (TG0F, 0, NotSerialized)
        {
            ADBG ("TG0F Start")
            ADBG (Concatenate ("DMA0 VDID -", ToHexString (\_SB.PC00.TDM0.VDID)))
            ADBG (Concatenate ("DMA0 PMST -", ToHexString (\_SB.PC00.TDM0.PMST)))
            ADBG (Concatenate ("DMA0 PMEE -", ToHexString (\_SB.PC00.TDM0.PMEE)))
            ADBG (Concatenate ("DMA0 PMES -", ToHexString (\_SB.PC00.TDM0.PMES)))
            ADBG (Concatenate ("DMA0 STAT -", ToHexString (\_SB.PC00.TDM0.STAT)))
            If ((\_SB.PC00.TDM0.VDID != 0xFFFFFFFF))
            {
                If ((\_SB.PC00.TDM0.STAT == One))
                {
                    \_SB.PC00.TDM0.D3CE ()
                    ADBG ("Let\'s push TBT RPs to D3Cold together")
                    If ((\_SB.PC00.TRP0.VDID != 0xFFFFFFFF))
                    {
                        ADBG (Concatenate ("PC00.TRP0.PDSX -", ToHexString (\_SB.PC00.TRP0.PDSX)))
                        If ((\_SB.PC00.TRP0.PDSX == One))
                        {
                            CTP0 = One
                        }

                        \_SB.PC00.TRP0.D3CE ()
                    }

                    If ((\_SB.PC00.TRP1.VDID != 0xFFFFFFFF))
                    {
                        ADBG (Concatenate ("PC00.TRP1.PDSX -", ToHexString (\_SB.PC00.TRP1.PDSX)))
                        If ((\_SB.PC00.TRP1.PDSX == One))
                        {
                            CTP0 = One
                        }

                        \_SB.PC00.TRP1.D3CE ()
                    }
                }
            }

            ADBG ("TG0F End")
        }

        Method (TG1N, 0, NotSerialized)
        {
            ADBG ("TG1N Start")
            ADBG (Concatenate ("DMA1 VDID -", ToHexString (\_SB.PC00.TDM1.VDID)))
            ADBG (Concatenate ("DMA1 PMST -", ToHexString (\_SB.PC00.TDM1.PMST)))
            ADBG (Concatenate ("DMA1 PMEE -", ToHexString (\_SB.PC00.TDM1.PMEE)))
            ADBG (Concatenate ("DMA1 PMES -", ToHexString (\_SB.PC00.TDM1.PMES)))
            ADBG (Concatenate ("DMA1 STAT -", ToHexString (\_SB.PC00.TDM1.STAT)))
            If ((\_SB.PC00.TDM1.VDID != 0xFFFFFFFF))
            {
                If ((\_SB.PC00.TDM1.STAT == Zero))
                {
                    \_SB.PC00.TDM1.D3CX ()
                    ADBG ("Let\'s bring TBT RPs out of D3Cold")
                    If ((\_SB.PC00.TRP2.VDID != 0xFFFFFFFF))
                    {
                        \_SB.PC00.TRP2.D3CX ()
                    }

                    If ((\_SB.PC00.TRP3.VDID != 0xFFFFFFFF))
                    {
                        \_SB.PC00.TRP3.D3CX ()
                    }

                    If ((\_SB.PC00.TDM1.ALCT == One))
                    {
                        If ((CTP1 == One))
                        {
                            \_SB.PC00.TDM1.CNTP ()
                            \_SB.PC00.TDM1.WACT = One
                            CTP1 = Zero
                        }

                        \_SB.PC00.TDM1.ALCT = Zero
                    }
                }
                Else
                {
                    ADBG ("Drop TG1N due to it is already exit D3 cold")
                }

                Sleep (ITRE)
            }

            ADBG ("TG1N End")
        }

        Method (TG1F, 0, NotSerialized)
        {
            ADBG ("TG1F Start")
            ADBG (Concatenate ("DMA1 VDID -", ToHexString (\_SB.PC00.TDM1.VDID)))
            ADBG (Concatenate ("DMA1 PMST -", ToHexString (\_SB.PC00.TDM1.PMST)))
            ADBG (Concatenate ("DMA1 PMEE -", ToHexString (\_SB.PC00.TDM1.PMEE)))
            ADBG (Concatenate ("DMA1 PMES -", ToHexString (\_SB.PC00.TDM1.PMES)))
            ADBG (Concatenate ("DMA1 STAT -", ToHexString (\_SB.PC00.TDM1.STAT)))
            If ((\_SB.PC00.TDM1.VDID != 0xFFFFFFFF))
            {
                If ((\_SB.PC00.TDM1.STAT == One))
                {
                    \_SB.PC00.TDM1.D3CE ()
                    ADBG ("Let\'s push TBT RPs to D3Cold together")
                    If ((\_SB.PC00.TRP2.VDID != 0xFFFFFFFF))
                    {
                        ADBG (Concatenate ("PC00.TRP2.PDSX -", ToHexString (\_SB.PC00.TRP2.PDSX)))
                        If ((\_SB.PC00.TRP2.PDSX == One))
                        {
                            CTP1 = One
                        }

                        \_SB.PC00.TRP2.D3CE ()
                    }

                    If ((\_SB.PC00.TRP3.VDID != 0xFFFFFFFF))
                    {
                        ADBG (Concatenate ("PC00.TRP3.PDSX -", ToHexString (\_SB.PC00.TRP3.PDSX)))
                        If ((\_SB.PC00.TRP3.PDSX == One))
                        {
                            CTP1 = One
                        }

                        \_SB.PC00.TRP3.D3CE ()
                    }
                }
            }

            ADBG ("TG1F End")
        }

        If (ITRT)
        {
            Method (TBON, 1, NotSerialized)
            {
                If (ITIM)
                {
                    If ((Arg0 == Zero))
                    {
                        ADBG ("TBT0 Group ON Start")
                        TG0N ()
                        ADBG ("TBT0 Group ON End")
                    }
                    Else
                    {
                        ADBG ("TBT1 Group ON Start")
                        TG1N ()
                        ADBG ("TBT1 Group ON End")
                    }
                }
                Else
                {
                    ADBG ("TBT IMR is not valid")
                }
            }

            Method (TBOF, 1, NotSerialized)
            {
                If (ITIM)
                {
                    If ((Arg0 == Zero))
                    {
                        ADBG ("TBT0 Group OFF Start")
                        ADBG (Concatenate ("Skip D3C entry? ", ToHexString (\_SB.PC00.TDM0.SD3C)))
                        If ((\_SB.PC00.TDM0.SD3C == Zero))
                        {
                            TG0F ()
                        }

                        ADBG ("TBT0 Group OFF End")
                    }
                    Else
                    {
                        ADBG ("TBT1 Group OFF Start")
                        ADBG (Concatenate ("Skip D3C entry? ", ToHexString (\_SB.PC00.TDM1.SD3C)))
                        If ((\_SB.PC00.TDM1.SD3C == Zero))
                        {
                            TG1F ()
                        }

                        ADBG ("TBT1 Group OFF End")
                    }
                }
                Else
                {
                    ADBG ("TBT IMR is not valid")
                }
            }

            PowerResource (TBT0, 0x00, 0x0001)
            {
                Method (_STA, 0, NotSerialized)  // _STA: Status
                {
                    ADBG (Concatenate ("DMA0 D3C STAT -", ToHexString (\_SB.PC00.TDM0.STAT)))
                    Return (\_SB.PC00.TDM0.STAT)
                }

                Method (_ON, 0, NotSerialized)  // _ON_: Power On
                {
                    ADBG ("TBT0 _ON")
                    If ((\_SB.PC00.TDM0.SCME == Zero))
                    {
                        TBON (Zero)
                    }
                }

                Method (_OFF, 0, NotSerialized)  // _OFF: Power Off
                {
                    ADBG ("TBT0 _OFF")
                    If ((\_SB.PC00.TDM0.SCME == Zero))
                    {
                        TBOF (Zero)
                    }
                }
            }

            PowerResource (TBT1, 0x00, 0x0001)
            {
                Method (_STA, 0, NotSerialized)  // _STA: Status
                {
                    ADBG (Concatenate ("DMA1 D3C STAT -", ToHexString (\_SB.PC00.TDM1.STAT)))
                    Return (\_SB.PC00.TDM1.STAT)
                }

                Method (_ON, 0, NotSerialized)  // _ON_: Power On
                {
                    ADBG ("TBT1 _ON")
                    If ((\_SB.PC00.TDM1.SCME == Zero))
                    {
                        TBON (One)
                    }
                }

                Method (_OFF, 0, NotSerialized)  // _OFF: Power Off
                {
                    ADBG ("TBT1 _OFF")
                    If ((\_SB.PC00.TDM1.SCME == Zero))
                    {
                        TBOF (One)
                    }
                }
            }
        }

        If (TRTD)
        {
            Method (TCON, 0, NotSerialized)
            {
                ADBG ("TCSS ON")
                ADBG ("TCON")
                If ((TD3C == One))
                {
                    ADBG (Concatenate ("TACK before D3 cold exit -", ToHexString (TACK)))
                    TD3C = Zero
                    Local0 = Zero
                    While (((TACK != Zero) && (Local0 < 0x64)))
                    {
                        Sleep (One)
                        Local0++
                    }

                    ADBG (Concatenate ("TACK after D3 cold exit -", ToHexString (TACK)))
                    ADBG (Concatenate ("Time for wait TACK - ", ToHexString (Local0)))
                    If ((Local0 == 0x64))
                    {
                        ADBG ("Error: Timeout occurred")
                    }
                    Else
                    {
                        ADBG ("D3Cold Exit")
                        If (((TDCE == One) && (IMRY == One)))
                        {
                            ADBG ("IOM.D3.TDCE")
                            \_SB.PC00.TXDC.SINT (TCIT, TCIR)
                            ADBG ("IOM.D3.TDCE Done")
                        }
                    }
                }
                Else
                {
                    ADBG ("Drop TCON due to it is already exit D3 cold")
                    Return (Zero)
                }

                ADBG ("TCSS ON End")
            }

            Method (TCOF, 0, NotSerialized)
            {
                ADBG ("TCSS OFF Start")
                ADBG ("TCOF")
                ADBG (Concatenate ("DMA0 Skip D3C entry? ", ToHexString (\_SB.PC00.TDM0.SD3C)))
                ADBG (Concatenate ("DMA1 Skip D3C entry? ", ToHexString (\_SB.PC00.TDM1.SD3C)))
                ADBG (Concatenate ("XHCI Skip D3C entry? ", ToHexString (\_SB.PC00.TXHC.SD3C)))
                If (((\_SB.PC00.TXHC.SD3C != Zero) || ((\_SB.PC00.TDM0.SD3C != Zero) || 
                    (\_SB.PC00.TDM1.SD3C != Zero))))
                {
                    Return (Zero)
                }

                ADBG ("D3Cold Entry")
                TD3C = One
                ADBG (Concatenate ("IOM D3Cold -", ToHexString (TD3C)))
                ADBG ("TCSS OFF End")
            }

            PowerResource (D3C, 0x00, 0x0000)
            {
                Name (STAT, One)
                Method (_STA, 0, NotSerialized)  // _STA: Status
                {
                    ADBG (Concatenate ("TCSS D3C STAT -", ToHexString (STAT)))
                    Return (STAT) /* \_SB_.PC00.D3C_.STAT */
                }

                Method (_ON, 0, NotSerialized)  // _ON_: Power On
                {
                    \_SB.PC00.TCON ()
                    STAT = One
                }

                Method (_OFF, 0, NotSerialized)  // _OFF: Power Off
                {
                    \_SB.PC00.TCOF ()
                    STAT = Zero
                }
            }
        }

        Device (TXHC)
        {
            Name (_ADR, 0x000D0000)  // _ADR: Address
            Name (_DDN, "ICL North XHCI controller")  // _DDN: DOS Device Name
            Name (_STR, Unicode ("ICL North XHCI controller"))  // _STR: Description String
            Name (DCPM, 0x04)
            Method (_STA, 0, NotSerialized)  // _STA: Status
            {
                ADBG (Concatenate ("TCSS xHCI _STA", ToHexString (THCE)))
                If ((THCE == One))
                {
                    Return (0x0F)
                }

                Return (Zero)
            }

            Method (BASE, 0, NotSerialized)
            {
                Local0 = (_ADR & 0x07)
                Local1 = ((_ADR >> 0x10) & 0x1F)
                Local2 = ((Local0 << 0x0C) + (Local1 << 0x0F))
                Local3 = (\_SB.PC00.GPCB () + Local2)
                ADBG (Concatenate ("CPU XHCI PCIe MMIO Address", ToHexString (Local3)))
                Return (Local3)
            }

            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
            {
                ADBG ("CPU XHCI _PS0 Start")
                If ((\_SB.PC00.TXHC.PMEE == One))
                {
                    ADBG ("Clear PME_EN of CPU XHCI")
                    \_SB.PC00.TXHC.PMEE = Zero
                }

                ADBG ("CPU XHCI _PS0 End")
            }

            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
            {
                ADBG ("CPU XHCI _PS3 Start")
                If ((\_SB.PC00.TXHC.PMEE == Zero))
                {
                    ADBG ("PME_EN was not set. Set PME_EN of CPU XHCI")
                    \_SB.PC00.TXHC.PMEE = One
                }

                ADBG ("CPU XHCI _PS3 End")
            }

            Method (_S0W, 0, NotSerialized)  // _S0W: S0 Device Wake State
            {
                If (TRTD)
                {
                    Return (0x04)
                }
                Else
                {
                    Return (0x03)
                }
            }

            Name (SD3C, Zero)
            If (TRTD)
            {
                Method (_PR0, 0, NotSerialized)  // _PR0: Power Resources for D0
                {
                    Return (Package (0x01)
                    {
                        \_SB.PC00.D3C
                    })
                }

                Method (_PR3, 0, NotSerialized)  // _PR3: Power Resources for D3hot
                {
                    Return (Package (0x01)
                    {
                        \_SB.PC00.D3C
                    })
                }
            }

            OperationRegion (XPRT, SystemMemory, BASE (), 0x0100)
            Field (XPRT, ByteAcc, NoLock, Preserve)
            {
                VDID,   32, 
                Offset (0x74), 
                D0D3,   2, 
                Offset (0x75), 
                PMEE,   1, 
                    ,   6, 
                PMES,   1
            }

            Name (XFLT, Zero)
            Method (_DSM, 4, Serialized)  // _DSM: Device-Specific Method
            {
                ADBG ("XHCI _DSM")
                If (PCIC (Arg0))
                {
                    Return (PCID (Arg0, Arg1, Arg2, Arg3))
                }

                Return (Buffer (One)
                {
                     0x00                                             // .
                })
            }

            Method (_S3D, 0, NotSerialized)  // _S3D: S3 Device State
            {
                Return (0x03)
            }

            Method (_S4D, 0, NotSerialized)  // _S4D: S4 Device State
            {
                Return (0x03)
            }

            Method (_S3W, 0, NotSerialized)  // _S3W: S3 Device Wake State
            {
                Return (0x03)
            }

            Method (_S4W, 0, NotSerialized)  // _S4W: S4 Device Wake State
            {
                Return (0x03)
            }

            Method (_PRW, 0, NotSerialized)  // _PRW: Power Resources for Wake
            {
                If (CondRefOf (GP1E))
                {
                    If ((GP1E != Zero))
                    {
                        Return (GPRW (0x80, 0x04))
                    }
                }

                Return (GPRW (0x6D, 0x04))
            }

            Method (_DSW, 3, NotSerialized)  // _DSW: Device Sleep Wake
            {
                ADBG ("TCSS XHCI _DSW Update TDM0 and TDM1 SD3C")
                ADBG (Concatenate ("Arg0 -", ToHexString (Arg0)))
                ADBG (Concatenate ("Arg1 -", ToHexString (Arg1)))
                ADBG (Concatenate ("Arg2 -", ToHexString (Arg2)))
                C2PM (Arg0, Arg1, Arg2, DCPM)
                SD3C = Arg1
                \_SB.PC00.TDM0.SD3C = Arg1
                \_SB.PC00.TDM1.SD3C = Arg1
                ADBG (Concatenate ("SD3C -", ToHexString (SD3C)))
            }

            Device (RHUB)
            {
                Name (_ADR, Zero)  // _ADR: Address
                Method (_DSM, 4, Serialized)  // _DSM: Device-Specific Method
                {
                    ADBG ("TCSS XHCI RHUB _DSM")
                    If ((Arg0 == ToUUID ("ce2ee385-00e6-48cb-9f05-2edb927c4899") /* USB Controller */))
                    {
                        If ((Arg2 == Zero))
                        {
                            ADBG ("TCSS XHCI RHUB F0")
                            Return (Buffer (One)
                            {
                                 0x81                                             // .
                            })
                        }

                        If ((Arg2 == 0x07))
                        {
                            ADBG ("TCSS XHCI RHUB F7")
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
                    Return (PCKG) /* \_SB_.PC00.TXHC.RHUB.TPLD.PCKG */
                }

                Method (TUPC, 3, Serialized)
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
                    PCKG [0x02] = Arg2
                    Return (PCKG) /* \_SB_.PC00.TXHC.RHUB.TUPC.PCKG */
                }

                Device (HS01)
                {
                    Name (_ADR, One)  // _ADR: Address
                    Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                    {
                        Return (CCPD (Zero, One))
                    }

                    Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                    {
                        Return (CCPD (Zero, 0x02))
                    }
                }

                Device (SS01)
                {
                    Name (_ADR, 0x02)  // _ADR: Address
                    Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                    {
                        Return (CCPD (One, One))
                    }

                    Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                    {
                        Return (CCPD (One, 0x02))
                    }

                    If (CondRefOf (U4CM))
                    {
                        If ((((U4CM & 0x07) == One) && (DME0 == One)))
                        {
                            ADBG (Concatenate ("CTEM -", ToHexString (CTEM)))
                            If (((CTEM & One) == One))
                            {
                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                {
                                    ADBG ("XHC SS01 _DSD")
                                    Return (Package (0x02)
                                    {
                                        ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                        Package (0x02)
                                        {
                                            Package (0x02)
                                            {
                                                "usb4-host-interface", 
                                                \_SB.PC00.TDM0
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

                    Method (_DSM, 4, NotSerialized)  // _DSM: Device-Specific Method
                    {
                        ADBG ("TCSS XHCI SS01 _DSM")
                        If ((Arg0 == ToUUID ("ce2ee385-00e6-48cb-9f05-2edb927c4899") /* USB Controller */))
                        {
                            ADBG ("TCSS XHCI SS01 GUID")
                            If ((Arg2 == Zero))
                            {
                                ADBG ("TCSS XHCI SS01 F0")
                                Return (Buffer (One)
                                {
                                     0x21                                             // !
                                })
                            }

                            If ((Arg2 == 0x05))
                            {
                                ADBG ("TCSS XHCI SS01 F5")
                                If ((Arg1 == Zero))
                                {
                                    If (((RTBM & One) == One))
                                    {
                                        ADBG ("XHC SS01 _DSM U1U2 disable")
                                        Return (One)
                                    }

                                    Return (Zero)
                                }
                            }
                        }

                        Return (Buffer (One)
                        {
                             0x00                                             // .
                        })
                    }
                }

                Device (SS02)
                {
                    Name (_ADR, 0x03)  // _ADR: Address
                    Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                    {
                        Return (CCPD (0x02, One))
                    }

                    Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                    {
                        Return (CCPD (0x02, 0x02))
                    }

                    If (CondRefOf (U4CM))
                    {
                        If ((((U4CM & 0x07) == One) && (DME0 == One)))
                        {
                            If (((CTEM & 0x02) == 0x02))
                            {
                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                {
                                    ADBG ("XHC SS02 _DSD")
                                    Return (Package (0x02)
                                    {
                                        ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                        Package (0x02)
                                        {
                                            Package (0x02)
                                            {
                                                "usb4-host-interface", 
                                                \_SB.PC00.TDM0
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

                    Method (_DSM, 4, NotSerialized)  // _DSM: Device-Specific Method
                    {
                        ADBG ("TCSS XHCI SS02 _DSM")
                        If ((Arg0 == ToUUID ("ce2ee385-00e6-48cb-9f05-2edb927c4899") /* USB Controller */))
                        {
                            ADBG ("TCSS XHCI SS02 GUID")
                            If ((Arg2 == Zero))
                            {
                                ADBG ("TCSS XHCI SS02 F0")
                                Return (Buffer (One)
                                {
                                     0x21                                             // !
                                })
                            }

                            If ((Arg2 == 0x05))
                            {
                                ADBG ("TCSS XHCI SS02 F5")
                                If ((Arg1 == Zero))
                                {
                                    If (((RTBM & 0x02) == 0x02))
                                    {
                                        ADBG ("XHC SS02 _DSM U1U2 disable")
                                        Return (One)
                                    }

                                    Return (Zero)
                                }
                            }
                        }

                        Return (Buffer (One)
                        {
                             0x00                                             // .
                        })
                    }
                }

                Device (SS03)
                {
                    Name (_ADR, 0x04)  // _ADR: Address
                    Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                    {
                        Return (CCPD (0x03, One))
                    }

                    Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                    {
                        Return (CCPD (0x03, 0x02))
                    }

                    If (CondRefOf (U4CM))
                    {
                        If ((((U4CM & 0x07) == One) && (DME1 == One)))
                        {
                            If (((CTEM & 0x04) == 0x04))
                            {
                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                {
                                    ADBG ("XHC SS03 _DSD")
                                    Return (Package (0x02)
                                    {
                                        ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                        Package (0x02)
                                        {
                                            Package (0x02)
                                            {
                                                "usb4-host-interface", 
                                                \_SB.PC00.TDM1
                                            }, 

                                            Package (0x02)
                                            {
                                                "usb4-port-number", 
                                                0x02
                                            }
                                        }
                                    })
                                }
                            }
                        }
                    }

                    Method (_DSM, 4, NotSerialized)  // _DSM: Device-Specific Method
                    {
                        ADBG ("TCSS XHCI SS03 _DSM")
                        If ((Arg0 == ToUUID ("ce2ee385-00e6-48cb-9f05-2edb927c4899") /* USB Controller */))
                        {
                            ADBG ("TCSS XHCI SS03 GUID")
                            If ((Arg2 == Zero))
                            {
                                ADBG ("TCSS XHCI SS03 F0")
                                Return (Buffer (One)
                                {
                                     0x21                                             // !
                                })
                            }

                            If ((Arg2 == 0x05))
                            {
                                ADBG ("TCSS XHCI SS03 F5")
                                If ((Arg1 == Zero))
                                {
                                    If (((RTBM & 0x04) == 0x04))
                                    {
                                        ADBG ("XHC SS03 _DSM U1U2 disable")
                                        Return (One)
                                    }

                                    Return (Zero)
                                }
                            }
                        }

                        Return (Buffer (One)
                        {
                             0x00                                             // .
                        })
                    }
                }

                Device (SS04)
                {
                    Name (_ADR, 0x05)  // _ADR: Address
                    Method (_UPC, 0, NotSerialized)  // _UPC: USB Port Capabilities
                    {
                        Return (CCPD (0x04, One))
                    }

                    Method (_PLD, 0, NotSerialized)  // _PLD: Physical Location of Device
                    {
                        Return (CCPD (0x04, 0x02))
                    }

                    If (CondRefOf (U4CM))
                    {
                        If ((((U4CM & 0x07) == One) && (DME1 == One)))
                        {
                            If (((CTEM & 0x08) == 0x08))
                            {
                                Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
                                {
                                    ADBG ("XHC SS04 _DSD")
                                    Return (Package (0x02)
                                    {
                                        ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                                        Package (0x02)
                                        {
                                            Package (0x02)
                                            {
                                                "usb4-host-interface", 
                                                \_SB.PC00.TDM1
                                            }, 

                                            Package (0x02)
                                            {
                                                "usb4-port-number", 
                                                0x03
                                            }
                                        }
                                    })
                                }
                            }
                        }
                    }

                    Method (_DSM, 4, NotSerialized)  // _DSM: Device-Specific Method
                    {
                        ADBG ("TCSS XHCI SS04 _DSM")
                        If ((Arg0 == ToUUID ("ce2ee385-00e6-48cb-9f05-2edb927c4899") /* USB Controller */))
                        {
                            ADBG ("TCSS XHCI SS04 GUID")
                            If ((Arg2 == Zero))
                            {
                                ADBG ("TCSS XHCI SS04 F0")
                                Return (Buffer (One)
                                {
                                     0x21                                             // !
                                })
                            }

                            If ((Arg2 == 0x05))
                            {
                                ADBG ("TCSS XHCI SS04 F5")
                                If ((Arg1 == Zero))
                                {
                                    If (((RTBM & 0x08) == 0x08))
                                    {
                                        ADBG ("XHC SS04 _DSM U1U2 disable")
                                        Return (One)
                                    }

                                    Return (Zero)
                                }
                            }
                        }

                        Return (Buffer (One)
                        {
                             0x00                                             // .
                        })
                    }
                }

                Method (_PS0, 0, Serialized)  // _PS0: Power State 0
                {
                    ADBG ("TCSS RHUB XHCI PS0")
                }

                Method (_PS2, 0, Serialized)  // _PS2: Power State 2
                {
                    ADBG ("TCSS RHUB XHCI PS2")
                }

                Method (_PS3, 0, Serialized)  // _PS3: Power State 3
                {
                    ADBG ("TCSS RHUB XHCI PS3")
                }
            }
        }

        If ((TDCE == One))
        {
            Device (TXDC)
            {
                Name (_ADR, 0x000D0001)  // _ADR: Address
                Name (_DDN, "ICL North XDCI controller")  // _DDN: DOS Device Name
                Name (_STR, Unicode ("ICL North XDCI controller"))  // _STR: Description String
                Name (DCPM, 0x05)
                Method (_STA, 0, NotSerialized)  // _STA: Status
                {
                    ADBG (Concatenate ("TCSS xDCI _STA", ToHexString (TDCE)))
                    If ((TDCE == One))
                    {
                        Return (0x0F)
                    }

                    Return (Zero)
                }

                Method (SINT, 2, Serialized)
                {
                    ADBG ("xDCI.SINT")
                    If ((IMRY == One))
                    {
                        Local0 = (Arg0 << 0x08)
                        Local1 = (Arg1 << 0x14)
                        Local0 += Local1
                        ADBG (Concatenate ("TCSS xDCI SINT", ToHexString (Local0)))
                        IMMC (0x02, Zero, One, Zero, Local0)
                    }
                    Else
                    {
                        ADBG ("IOM not ready")
                    }

                    ADBG ("xDCI.SINT Done")
                }

                Method (_S0W, 0, NotSerialized)  // _S0W: S0 Device Wake State
                {
                    Return (0x03)
                }

                OperationRegion (XDCS, PCI_Config, Zero, 0x0100)
                Field (XDCS, DWordAcc, NoLock, Preserve)
                {
                    DVID,   16, 
                    Offset (0x10), 
                    XDCB,   64
                }

                Field (XDCS, ByteAcc, NoLock, Preserve)
                {
                    Offset (0x84), 
                    D0I3,   2
                }

                Method (XDBA, 0, NotSerialized)
                {
                    Return ((^XDCB & 0xFFFFFFFFFFFFFF00))
                }

                OperationRegion (XDMM, SystemMemory, XDBA (), 0x00110000)
                Field (XDMM, WordAcc, NoLock, Preserve)
                {
                    Offset (0xC110), 
                    GCTL,   32, 
                    Offset (0x10F810), 
                    Offset (0x10F811), 
                    U2CP,   2, 
                    U3CP,   2, 
                    Offset (0x10F818), 
                    PUPS,   2, 
                        ,   1, 
                    PURC,   1, 
                    Offset (0x10F81A), 
                    Offset (0x10F81C), 
                        ,   3, 
                    UXPE,   2, 
                    Offset (0x10F81E)
                }

                Method (_DSM, 4, Serialized)  // _DSM: Device-Specific Method
                {
                    If (PCIC (Arg0))
                    {
                        Return (PCID (Arg0, Arg1, Arg2, Arg3))
                    }

                    ADBG ("TXDC DSM")
                    If ((Arg0 == ToUUID ("732b85d5-b7a7-4a1b-9ba0-4bbd00ffd511") /* Unknown UUID */))
                    {
                        If ((Arg1 == One))
                        {
                            Method (SPPT, 2, Serialized)
                            {
                                ADBG ("TXDC Set PMU Power State")
                                Local1 = Arg0
                                Local2 = Arg1
                                If ((Local1 == Zero))
                                {
                                    ADBG ("TXDC PMU D0")
                                    UXPE = Zero
                                    Local0 = Zero
                                    While ((Local0 < 0x0A))
                                    {
                                        Stall (0x64)
                                        Local0++
                                    }

                                    PUPS = Zero
                                    Local0 = Zero
                                    While ((Local0 < 0x07D0))
                                    {
                                        Stall (0x64)
                                        If (((U2CP == Zero) && (U3CP == Zero)))
                                        {
                                            Break
                                        }

                                        Local0++
                                    }

                                    If ((U2CP != Zero))
                                    {
                                        ADBG ("TXDC U2 not in D0")
                                    }

                                    If ((U3CP != Zero))
                                    {
                                        ADBG ("TXDC U3 not in D0")
                                    }

                                    Return (Zero)
                                }

                                If ((Local1 == 0x03))
                                {
                                    ADBG ("TXDC PMU D3")
                                    If ((U2CP != Zero))
                                    {
                                        ADBG ("TXDC U2 not in D0")
                                    }

                                    If ((U3CP != Zero))
                                    {
                                        ADBG ("TXDC U3 not in D0")
                                    }

                                    PUPS = 0x03
                                    Local0 = Zero
                                    While ((Local0 < 0x07D0))
                                    {
                                        Stall (0x64)
                                        If (((U2CP == 0x03) && (U3CP == 0x03)))
                                        {
                                            Break
                                        }

                                        Local0++
                                    }

                                    If ((U2CP != 0x03))
                                    {
                                        ADBG ("TXDC U2 not in D3")
                                    }

                                    If ((U3CP != 0x03))
                                    {
                                        ADBG ("TXDC U3 not in D3")
                                    }

                                    UXPE = Local2
                                    Return (Zero)
                                }

                                Return (Zero)
                            }

                            Switch (ToInteger (Arg2))
                            {
                                Case (Zero)
                                {
                                    ADBG ("TXDC Fn0")
                                    Return (Buffer (0x02)
                                    {
                                         0xF3, 0x03                                       // ..
                                    })
                                }
                                Case (One)
                                {
                                    ADBG ("TXDC Fn1")
                                    Return (One)
                                }
                                Case (0x04)
                                {
                                    ADBG ("TXDC Fn4")
                                    Local1 = DerefOf (Arg3 [Zero])
                                    ADBG (Local1)
                                    SPPT (Local1, Zero)
                                }
                                Case (0x05)
                                {
                                    ADBG ("TXDC Fn5")
                                    If (CondRefOf (XDAT))
                                    {
                                        If ((XDAT () == One))
                                        {
                                            ADBG ("USB Attach")
                                            Notify (\_SB.PC00.TXDC, 0x80) // Status Change
                                        }
                                        Else
                                        {
                                            ADBG ("USB Detach")
                                            Notify (\_SB.PC00.TXDC, 0x81) // Information Change
                                        }
                                    }

                                    Return (Zero)
                                }
                                Case (0x06)
                                {
                                    ADBG ("TXDC Fn6")
                                    If ((OTHC == Zero))
                                    {
                                        CSFR = One
                                        Local0 = Zero
                                        While ((Local0 < 0x64))
                                        {
                                            If ((CSFR == Zero))
                                            {
                                                Break
                                            }

                                            Sleep (One)
                                        }
                                    }

                                    Return (Zero)
                                }
                                Case (0x07)
                                {
                                    ADBG ("TXDC Fn7")
                                    Local0 = PUPS /* \_SB_.PC00.TXDC.PUPS */
                                    Return (Local0)
                                }
                                Case (0x08)
                                {
                                    ADBG ("TXDC Fn8")
                                    Return (One)
                                }
                                Case (0x09)
                                {
                                    ADBG ("TXDC Fn9")
                                    Local1 = (TCDS & 0xFFF80000)
                                    Local1 >>= 0x13
                                    ADBG (Concatenate ("CPU XDCI: Func9 Return Val = ", ToHexString (Local1)))
                                    Return (Local1)
                                }

                            }
                        }
                    }

                    Return (Buffer (One)
                    {
                         0x00                                             // .
                    })
                }

                Method (_PRW, 0, NotSerialized)  // _PRW: Power Resources for Wake
                {
                    If (CondRefOf (GP1E))
                    {
                        If ((GP1E != Zero))
                        {
                            Return (GPRW (0x80, 0x04))
                        }
                    }

                    Return (GPRW (0x6D, 0x04))
                }

                Method (_DSW, 3, NotSerialized)  // _DSW: Device Sleep Wake
                {
                    C2PM (Arg0, Arg1, Arg2, DCPM)
                }
            }
        }

        Device (TDM0)
        {
            Name (_ADR, 0x000D0002)  // _ADR: Address
            Name (_DDN, "ICL TBT DMA0 controller")  // _DDN: DOS Device Name
            Name (_STR, Unicode ("ICL TBT DMA0 controller"))  // _STR: Description String
            Name (DUID, Zero)
            Name (DCPM, 0x06)
            Method (_STA, 0, NotSerialized)  // _STA: Status
            {
                ADBG (Concatenate ("TCSS DMA0 _STA", ToHexString (DME0)))
                If ((DME0 == One))
                {
                    Return (0x0F)
                }

                Return (Zero)
            }

            Method (BASE, 0, NotSerialized)
            {
                Local0 = (\_SB.PC00.GPCB () + 0x0006A000)
                ADBG (Concatenate ("TDM0 MMIO Address", ToHexString (Local0)))
                Return (Local0)
            }

            OperationRegion (DPME, SystemMemory, BASE (), 0x0100)
            Field (DPME, AnyAcc, NoLock, Preserve)
            {
                VDID,   32, 
                Offset (0x84), 
                PMST,   2, 
                Offset (0x85), 
                PMEE,   1, 
                    ,   6, 
                PMES,   1, 
                Offset (0xFC), 
                DD3E,   1, 
                DFPE,   1, 
                Offset (0xFF), 
                DMAD,   8
            }

            Name (STAT, One)
            Name (ALCT, Zero)
            Name (WACT, Zero)
            Name (SCME, Zero)
            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
            {
                Sleep (0x03)
                ADBG (Concatenate ("TBT DMA PS0 Start, DUID -", ToHexString (DUID)))
                ADBG (Concatenate ("DMA VDID -", ToHexString (VDID)))
                ADBG (Concatenate ("PMST -", ToHexString (PMST)))
                ADBG (Concatenate ("PMEE -", ToHexString (PMEE)))
                ADBG (Concatenate ("PMES -", ToHexString (PMES)))
                ADBG (Concatenate ("SCME -", ToHexString (SCME)))
                If ((SCME == One))
                {
                    TBON (DUID)
                }
                ElseIf ((WACT == One))
                {
                    WACT = 0x02
                    WFCC (ITCT)
                    WACT = Zero
                }
                ElseIf ((WACT == 0x02))
                {
                    ADBG ("Wait until other _PS0 get response")
                    While ((WACT != Zero))
                    {
                        Sleep (0x05)
                    }

                    ADBG ("Other _PS0 got response")
                }

                ADBG (Concatenate ("TBT DMA PS0 End, DUID -", ToHexString (DUID)))
            }

            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
            {
                ADBG (Concatenate ("TBT DMA PS3 Start, DUID -", ToHexString (DUID)))
                ADBG (Concatenate ("DMA VDID -", ToHexString (VDID)))
                ADBG (Concatenate ("PMST -", ToHexString (PMST)))
                ADBG (Concatenate ("PMEE -", ToHexString (PMEE)))
                ADBG (Concatenate ("PMES -", ToHexString (PMES)))
                ADBG (Concatenate ("SCME -", ToHexString (SCME)))
                If ((SCME == One))
                {
                    TBOF (DUID)
                }

                ADBG (Concatenate ("TBT DMA PS3 End, DUID -", ToHexString (DUID)))
            }

            Method (_S0W, 0, NotSerialized)  // _S0W: S0 Device Wake State
            {
                ADBG ("TDMA S0W")
                If (ITRT)
                {
                    Return (0x04)
                }
                Else
                {
                    Return (0x03)
                }
            }

            If (ITRT)
            {
                Method (_PR0, 0, NotSerialized)  // _PR0: Power Resources for D0
                {
                    If (TRTD)
                    {
                        If (SCME)
                        {
                            Return (Package (0x01)
                            {
                                \_SB.PC00.D3C
                            })
                        }

                        If ((DUID == Zero))
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT0
                            })
                        }
                        Else
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT1
                            })
                        }
                    }

                    If ((DUID == Zero))
                    {
                        Return (Package (0x01)
                        {
                            \_SB.PC00.TBT0
                        })
                    }

                    Return (Package (0x01)
                    {
                        \_SB.PC00.TBT1
                    })
                }

                Method (_PR3, 0, NotSerialized)  // _PR3: Power Resources for D3hot
                {
                    If (TRTD)
                    {
                        If (SCME)
                        {
                            Return (Package (0x01)
                            {
                                \_SB.PC00.D3C
                            })
                        }

                        If ((DUID == Zero))
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT0
                            })
                        }
                        Else
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT1
                            })
                        }
                    }

                    If ((DUID == Zero))
                    {
                        Return (Package (0x01)
                        {
                            \_SB.PC00.TBT0
                        })
                    }

                    Return (Package (0x01)
                    {
                        \_SB.PC00.TBT1
                    })
                }
            }

            Method (D3CX, 0, Serialized)
            {
                ADBG ("Disable DMA RTD3 bit")
                ADBG (Concatenate ("DUID -", ToHexString (DUID)))
                DD3E = Zero
                STAT = One
            }

            Method (D3CE, 0, Serialized)
            {
                ADBG ("Enable DMA RTD3 bit")
                ADBG (Concatenate ("DUID -", ToHexString (DUID)))
                DD3E = One
                STAT = Zero
                If ((SCME == Zero))
                {
                    ALCT = One
                }
            }

            Name (SD3C, Zero)
            Method (_DSW, 3, NotSerialized)  // _DSW: Device Sleep Wake
            {
                ADBG (Concatenate ("TCSS DMA _DSW DUID -", ToHexString (DUID)))
                ADBG (Concatenate ("Arg0 -", ToHexString (Arg0)))
                ADBG (Concatenate ("Arg1 -", ToHexString (Arg1)))
                SD3C = Arg1
                ADBG (Concatenate ("SD3C -", ToHexString (SD3C)))
            }

            Method (_PRW, 0, NotSerialized)  // _PRW: Power Resources for Wake
            {
                If (CondRefOf (GP1E))
                {
                    If ((GP1E != Zero))
                    {
                        Return (GPRW (0x80, 0x04))
                    }
                }

                Return (GPRW (0x6D, 0x04))
            }

            Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
            {
                ADBG (Concatenate ("TBT _DSD IMR_VALID =", ToHexString (ITIM)))
                ADBG (Concatenate ("TBT _DSD WAKE_SUPPORTED =", ToHexString (TIVS)))
                If (ITIM)
                {
                    If (TIVS)
                    {
                        Return (Package (0x04)
                        {
                            ToUUID ("c44d002f-69f9-4e7d-a904-a7baabdf43f7") /* Unknown UUID */, 
                            Package (0x01)
                            {
                                Package (0x02)
                                {
                                    "IMR_VALID", 
                                    One
                                }
                            }, 

                            ToUUID ("6c501103-c189-4296-ba72-9bf5a26ebe5d") /* Unknown UUID */, 
                            Package (0x01)
                            {
                                Package (0x02)
                                {
                                    "WAKE_SUPPORTED", 
                                    One
                                }
                            }
                        })
                    }
                    Else
                    {
                        Return (Package (0x04)
                        {
                            ToUUID ("c44d002f-69f9-4e7d-a904-a7baabdf43f7") /* Unknown UUID */, 
                            Package (0x01)
                            {
                                Package (0x02)
                                {
                                    "IMR_VALID", 
                                    One
                                }
                            }, 

                            ToUUID ("6c501103-c189-4296-ba72-9bf5a26ebe5d") /* Unknown UUID */, 
                            Package (0x01)
                            {
                                Package (0x02)
                                {
                                    "WAKE_SUPPORTED", 
                                    Zero
                                }
                            }
                        })
                    }
                }
                ElseIf (TIVS)
                {
                    Return (Package (0x04)
                    {
                        ToUUID ("c44d002f-69f9-4e7d-a904-a7baabdf43f7") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "IMR_VALID", 
                                Zero
                            }
                        }, 

                        ToUUID ("6c501103-c189-4296-ba72-9bf5a26ebe5d") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "WAKE_SUPPORTED", 
                                One
                            }
                        }
                    })
                }
                Else
                {
                    Return (Package (0x04)
                    {
                        ToUUID ("c44d002f-69f9-4e7d-a904-a7baabdf43f7") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "IMR_VALID", 
                                Zero
                            }
                        }, 

                        ToUUID ("6c501103-c189-4296-ba72-9bf5a26ebe5d") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "WAKE_SUPPORTED", 
                                Zero
                            }
                        }
                    })
                }
            }

            Method (_DSM, 4, Serialized)  // _DSM: Device-Specific Method
            {
                If ((Arg0 == ToUUID ("197db5e0-f095-4f33-b915-71dd70833e55") /* Unknown UUID */))
                {
                    If ((Arg2 == Zero))
                    {
                        If ((Arg1 == One))
                        {
                            Return (Buffer (One)
                            {
                                 0x07                                             // .
                            })
                        }
                        Else
                        {
                            Return (Buffer (One)
                            {
                                 0x01                                             // .
                            })
                        }
                    }

                    If ((Arg2 == One))
                    {
                        Name (PCPN, Buffer (One)
                        {
                             0x00                                             // .
                        })
                        CreateBitField (PCPN, Zero, PCP0)
                        CreateBitField (PCPN, One, PCP1)
                        If ((DUID == Zero))
                        {
                            If ((TRE0 == One))
                            {
                                PCP0 = One
                            }

                            If ((TRE1 == One))
                            {
                                PCP1 = One
                            }
                        }
                        Else
                        {
                            If ((TRE2 == One))
                            {
                                PCP0 = One
                            }

                            If ((TRE3 == One))
                            {
                                PCP1 = One
                            }
                        }

                        Return (PCPN) /* \_SB_.PC00.TDM0._DSM.PCPN */
                    }

                    If ((Arg2 == 0x02))
                    {
                        Name (TCAP, Buffer (One)
                        {
                             0x00                                             // .
                        })
                        CreateBitField (TCAP, Zero, IMRV)
                        CreateBitField (TCAP, One, WAKE)
                        CreateBitField (TCAP, 0x02, TVDS)
                        If (ITIM)
                        {
                            IMRV = One
                        }

                        If (TIVS)
                        {
                            WAKE = One
                        }

                        If (ITVD)
                        {
                            TVDS = One
                        }

                        Return (TCAP) /* \_SB_.PC00.TDM0._DSM.TCAP */
                    }
                }
                ElseIf ((Arg0 == Buffer (0x10)
                        {
                            /* 0000 */  0x32, 0x52, 0x42, 0x1B, 0x4C, 0x15, 0x34, 0x4F,  // 2RB.L.4O
                            /* 0008 */  0x7B, 0x9E, 0xB6, 0x5C, 0x94, 0x64, 0x94, 0xA3   // {..\.d..
                        }))
                {
                    Switch (ToInteger (Arg2))
                    {
                        Case (Zero)
                        {
                            ADBG (Concatenate ("TBT DMA _DSM DUID:", ToHexString (DUID)))
                            Switch (ToInteger (Arg1))
                            {
                                Case (One)
                                {
                                    Return (Buffer (One)
                                    {
                                         0x03                                             // .
                                    })
                                }

                            }

                            ADBG ("return buffer 0x00")
                            Return (Buffer (One)
                            {
                                 0x00                                             // .
                            })
                        }
                        Case (One)
                        {
                            ADBG (Concatenate ("TBT DMA _DSM Func 1 DUID:", ToHexString (DUID)))
                            If ((DUID == One))
                            {
                                ADBG (Concatenate ("ITM1:", ToHexString (ITM1)))
                                Return (ITM1) /* External reference */
                            }
                            Else
                            {
                                ADBG (Concatenate ("ITM0:", ToHexString (ITM0)))
                                Return (ITM0) /* External reference */
                            }
                        }

                    }
                }

                Return (Buffer (One)
                {
                     0x00                                             // .
                })
            }

            Method (STCM, 2, Serialized)
            {
                ADBG (Concatenate ("STCM DUID = ", ToHexString (DUID)))
                ADBG (Concatenate ("Requested CM mode = ", Arg0))
                ADBG (Concatenate ("Pre-boot CM Info  = ", ToHexString (Arg1)))
                Local0 = (Arg1 & 0x07)
                Local1 = One
                If (((Arg1 & 0x80) != 0x80))
                {
                    ADBG ("U4CM valid bit is not set!")
                    Return (0xFF)
                }
                Else
                {
                    SCME = Local0
                    If ((Arg0 == Local0))
                    {
                        ADBG ("Pre-boot CM mode aligns with OS preferred CM mode")
                        Local1 = Zero
                    }
                }

                If ((Local1 == One))
                {
                    If ((CHCM (Arg0) == Zero))
                    {
                        ADBG (Concatenate ("Switch CM mode success, request mode = ", Arg0))
                        SCME = Arg0
                    }
                    Else
                    {
                        ADBG (Concatenate ("Switch CM mode failure, request mode = ", Arg0))
                        Return (0xFF)
                    }
                }

                ADBG (Concatenate ("SCME - ", ToHexString (SCME)))
                Return (SCME) /* \_SB_.PC00.TDM0.SCME */
            }

            Field (DPME, DWordAcc, NoLock, Preserve)
            {
                Offset (0xC8), 
                    ,   31, 
                INFR,   1, 
                Offset (0xEC), 
                TB2P,   32, 
                P2TB,   32
            }

            Method (ITMB, 1, Serialized)
            {
                ADBG (ToHexString (_DDN))
                ADBG (Concatenate ("MailBox Cmd ", ToHexString ((Arg0 >> One))))
                Local0 = (Arg0 | One)
                P2TB = Local0
                ADBG ("Command is sent")
            }

            Method (WFCC, 1, Serialized)
            {
                WTBS (Arg0)
                P2TB = Zero
                WTBC (Arg0)
            }

            Method (WTBS, 1, Serialized)
            {
                Local0 = Arg0
                ADBG (Concatenate ("Wait TB2P to be set, DUID ", ToHexString (DUID)))
                While ((Local0 > Zero))
                {
                    If ((TB2P & One))
                    {
                        ADBG (Concatenate ("TB2P is set, DUID ", ToHexString (DUID)))
                        ADBG (Concatenate ("Wait count = ", ToHexString ((Arg0 - Local0))))
                        Break
                    }

                    Local0--
                    Sleep (One)
                }

                If ((Local0 == Zero))
                {
                    ADBG (Concatenate ("Command Timeout, DUID ", ToHexString (DUID)))
                    ADBG (Concatenate ("Timeout Value ", ToHexString (Arg0)))
                }

                Return (Local0)
            }

            Method (WTBC, 1, Serialized)
            {
                Local0 = Arg0
                ADBG (Concatenate ("Wait TB2P to be clear, DUID ", ToHexString (DUID)))
                While ((Local0 > Zero))
                {
                    If (((TB2P & One) != One))
                    {
                        ADBG (Concatenate ("TB2P is clear, DUID ", ToHexString (DUID)))
                        Break
                    }

                    Local0--
                    Sleep (One)
                }

                If ((Local0 == Zero))
                {
                    ADBG (Concatenate ("Command Timeout, DUID ", ToHexString (DUID)))
                    ADBG (Concatenate ("Timeout Value ", ToHexString (Arg0)))
                }
            }

            Method (CNTP, 0, Serialized)
            {
                ADBG (Concatenate ("TCSS Send ITBT CONNECT TOPOLOGY Cmd Start, DUID -", ToHexString (DUID)))
                Local0 = Zero
                If ((DFPE == Zero))
                {
                    DMAD = 0x22
                    DFPE = One
                    While (((INFR == Zero) && (Local0 < ITFP)))
                    {
                        Sleep (One)
                        Local0++
                    }
                }

                If ((Local0 != ITFP))
                {
                    ITMB (0x3E)
                }
                Else
                {
                    ADBG ("Force Power Timeout occurred")
                }

                If ((DFPE != Zero))
                {
                    ADBG ("Unset Force Power")
                    DFPE = Zero
                }

                ADBG (Concatenate ("TCSS Send ITBT CONNECT TOPOLOGY Cmd End, DUID -", ToHexString (DUID)))
            }

            Method (CMMB, 1, Serialized)
            {
                If ((Arg0 == Zero))
                {
                    ADBG ("Set FW CM mode")
                    ITMB (0x44)
                }
                ElseIf ((Arg0 == One))
                {
                    ADBG ("Set SW CM mode")
                    ITMB (0x46)
                }

                Local0 = WTBS (0x01F4)
                If ((Local0 != Zero))
                {
                    ADBG ("Set CM mode done bit is set")
                    Local1 = Zero
                }
                Else
                {
                    ADBG ("Set CM mode command timeout")
                    Local1 = 0xFF
                }

                P2TB = Zero
                Return (Local1)
            }

            Method (CHCM, 1, Serialized)
            {
                ADBG (Concatenate ("iTBT CHCM - Switch CM mode to ", Arg0))
                If ((Arg0 > One))
                {
                    ADBG ("Invalid CM mode value")
                    Return (0xFF)
                }

                Local0 = Zero
                If ((DFPE == Zero))
                {
                    ADBG ("Set Force Power")
                    DMAD = 0x22
                    DFPE = One
                    While (((INFR == Zero) && (Local0 < ITFP)))
                    {
                        Sleep (One)
                        Local0++
                    }
                }
                Else
                {
                    ADBG ("Force Power has been set")
                }

                If ((Local0 != ITFP))
                {
                    Local1 = CMMB (Arg0)
                    If (((Arg0 == Zero) && (Local1 == Zero)))
                    {
                        ADBG ("Send Connect Topology command")
                        ITMB (0x3E)
                        Local2 = WTBS (ITCT)
                        If ((Local2 != Zero))
                        {
                            ADBG ("Connect topology command done bit is set")
                            Local1 = Zero
                        }
                        Else
                        {
                            ADBG ("Connect topology command timeout!")
                            Local1 = 0xFF
                        }

                        P2TB = Zero
                    }
                }
                Else
                {
                    ADBG ("Force Power Timeout occurred")
                    Local1 = 0xFF
                }

                If ((DFPE != Zero))
                {
                    ADBG ("Unset Force Power")
                    DFPE = Zero
                }

                ADBG (Concatenate ("iTBT change CM mode status = ", Local1))
                Return (Local1)
            }
        }

        Device (TDM1)
        {
            Name (_ADR, 0x000D0003)  // _ADR: Address
            Name (_DDN, "ICL TBT DMA1 controller")  // _DDN: DOS Device Name
            Name (_STR, Unicode ("ICL TBT DMA1 controller"))  // _STR: Description String
            Name (DUID, One)
            Name (DCPM, 0x07)
            Method (_STA, 0, NotSerialized)  // _STA: Status
            {
                ADBG (Concatenate ("TCSS DMA1 _STA", ToHexString (DME1)))
                If ((DME1 == One))
                {
                    Return (0x0F)
                }

                Return (Zero)
            }

            Method (BASE, 0, NotSerialized)
            {
                Local0 = (\_SB.PC00.GPCB () + 0x0006B000)
                ADBG (Concatenate ("TDM1 MMIO Address", ToHexString (Local0)))
                Return (Local0)
            }

            OperationRegion (DPME, SystemMemory, BASE (), 0x0100)
            Field (DPME, AnyAcc, NoLock, Preserve)
            {
                VDID,   32, 
                Offset (0x84), 
                PMST,   2, 
                Offset (0x85), 
                PMEE,   1, 
                    ,   6, 
                PMES,   1, 
                Offset (0xFC), 
                DD3E,   1, 
                DFPE,   1, 
                Offset (0xFF), 
                DMAD,   8
            }

            Name (STAT, One)
            Name (ALCT, Zero)
            Name (WACT, Zero)
            Name (SCME, Zero)
            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
            {
                Sleep (0x03)
                ADBG (Concatenate ("TBT DMA PS0 Start, DUID -", ToHexString (DUID)))
                ADBG (Concatenate ("DMA VDID -", ToHexString (VDID)))
                ADBG (Concatenate ("PMST -", ToHexString (PMST)))
                ADBG (Concatenate ("PMEE -", ToHexString (PMEE)))
                ADBG (Concatenate ("PMES -", ToHexString (PMES)))
                ADBG (Concatenate ("SCME -", ToHexString (SCME)))
                If ((SCME == One))
                {
                    TBON (DUID)
                }
                ElseIf ((WACT == One))
                {
                    WACT = 0x02
                    WFCC (ITCT)
                    WACT = Zero
                }
                ElseIf ((WACT == 0x02))
                {
                    ADBG ("Wait until other _PS0 get response")
                    While ((WACT != Zero))
                    {
                        Sleep (0x05)
                    }

                    ADBG ("Other _PS0 got response")
                }

                ADBG (Concatenate ("TBT DMA PS0 End, DUID -", ToHexString (DUID)))
            }

            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
            {
                ADBG (Concatenate ("TBT DMA PS3 Start, DUID -", ToHexString (DUID)))
                ADBG (Concatenate ("DMA VDID -", ToHexString (VDID)))
                ADBG (Concatenate ("PMST -", ToHexString (PMST)))
                ADBG (Concatenate ("PMEE -", ToHexString (PMEE)))
                ADBG (Concatenate ("PMES -", ToHexString (PMES)))
                ADBG (Concatenate ("SCME -", ToHexString (SCME)))
                If ((SCME == One))
                {
                    TBOF (DUID)
                }

                ADBG (Concatenate ("TBT DMA PS3 End, DUID -", ToHexString (DUID)))
            }

            Method (_S0W, 0, NotSerialized)  // _S0W: S0 Device Wake State
            {
                ADBG ("TDMA S0W")
                If (ITRT)
                {
                    Return (0x04)
                }
                Else
                {
                    Return (0x03)
                }
            }

            If (ITRT)
            {
                Method (_PR0, 0, NotSerialized)  // _PR0: Power Resources for D0
                {
                    If (TRTD)
                    {
                        If (SCME)
                        {
                            Return (Package (0x01)
                            {
                                \_SB.PC00.D3C
                            })
                        }

                        If ((DUID == Zero))
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT0
                            })
                        }
                        Else
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT1
                            })
                        }
                    }

                    If ((DUID == Zero))
                    {
                        Return (Package (0x01)
                        {
                            \_SB.PC00.TBT0
                        })
                    }

                    Return (Package (0x01)
                    {
                        \_SB.PC00.TBT1
                    })
                }

                Method (_PR3, 0, NotSerialized)  // _PR3: Power Resources for D3hot
                {
                    If (TRTD)
                    {
                        If (SCME)
                        {
                            Return (Package (0x01)
                            {
                                \_SB.PC00.D3C
                            })
                        }

                        If ((DUID == Zero))
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT0
                            })
                        }
                        Else
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT1
                            })
                        }
                    }

                    If ((DUID == Zero))
                    {
                        Return (Package (0x01)
                        {
                            \_SB.PC00.TBT0
                        })
                    }

                    Return (Package (0x01)
                    {
                        \_SB.PC00.TBT1
                    })
                }
            }

            Method (D3CX, 0, Serialized)
            {
                ADBG ("Disable DMA RTD3 bit")
                ADBG (Concatenate ("DUID -", ToHexString (DUID)))
                DD3E = Zero
                STAT = One
            }

            Method (D3CE, 0, Serialized)
            {
                ADBG ("Enable DMA RTD3 bit")
                ADBG (Concatenate ("DUID -", ToHexString (DUID)))
                DD3E = One
                STAT = Zero
                If ((SCME == Zero))
                {
                    ALCT = One
                }
            }

            Name (SD3C, Zero)
            Method (_DSW, 3, NotSerialized)  // _DSW: Device Sleep Wake
            {
                ADBG (Concatenate ("TCSS DMA _DSW DUID -", ToHexString (DUID)))
                ADBG (Concatenate ("Arg0 -", ToHexString (Arg0)))
                ADBG (Concatenate ("Arg1 -", ToHexString (Arg1)))
                SD3C = Arg1
                ADBG (Concatenate ("SD3C -", ToHexString (SD3C)))
            }

            Method (_PRW, 0, NotSerialized)  // _PRW: Power Resources for Wake
            {
                If (CondRefOf (GP1E))
                {
                    If ((GP1E != Zero))
                    {
                        Return (GPRW (0x80, 0x04))
                    }
                }

                Return (GPRW (0x6D, 0x04))
            }

            Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
            {
                ADBG (Concatenate ("TBT _DSD IMR_VALID =", ToHexString (ITIM)))
                ADBG (Concatenate ("TBT _DSD WAKE_SUPPORTED =", ToHexString (TIVS)))
                If (ITIM)
                {
                    If (TIVS)
                    {
                        Return (Package (0x04)
                        {
                            ToUUID ("c44d002f-69f9-4e7d-a904-a7baabdf43f7") /* Unknown UUID */, 
                            Package (0x01)
                            {
                                Package (0x02)
                                {
                                    "IMR_VALID", 
                                    One
                                }
                            }, 

                            ToUUID ("6c501103-c189-4296-ba72-9bf5a26ebe5d") /* Unknown UUID */, 
                            Package (0x01)
                            {
                                Package (0x02)
                                {
                                    "WAKE_SUPPORTED", 
                                    One
                                }
                            }
                        })
                    }
                    Else
                    {
                        Return (Package (0x04)
                        {
                            ToUUID ("c44d002f-69f9-4e7d-a904-a7baabdf43f7") /* Unknown UUID */, 
                            Package (0x01)
                            {
                                Package (0x02)
                                {
                                    "IMR_VALID", 
                                    One
                                }
                            }, 

                            ToUUID ("6c501103-c189-4296-ba72-9bf5a26ebe5d") /* Unknown UUID */, 
                            Package (0x01)
                            {
                                Package (0x02)
                                {
                                    "WAKE_SUPPORTED", 
                                    Zero
                                }
                            }
                        })
                    }
                }
                ElseIf (TIVS)
                {
                    Return (Package (0x04)
                    {
                        ToUUID ("c44d002f-69f9-4e7d-a904-a7baabdf43f7") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "IMR_VALID", 
                                Zero
                            }
                        }, 

                        ToUUID ("6c501103-c189-4296-ba72-9bf5a26ebe5d") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "WAKE_SUPPORTED", 
                                One
                            }
                        }
                    })
                }
                Else
                {
                    Return (Package (0x04)
                    {
                        ToUUID ("c44d002f-69f9-4e7d-a904-a7baabdf43f7") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "IMR_VALID", 
                                Zero
                            }
                        }, 

                        ToUUID ("6c501103-c189-4296-ba72-9bf5a26ebe5d") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "WAKE_SUPPORTED", 
                                Zero
                            }
                        }
                    })
                }
            }

            Method (_DSM, 4, Serialized)  // _DSM: Device-Specific Method
            {
                If ((Arg0 == ToUUID ("197db5e0-f095-4f33-b915-71dd70833e55") /* Unknown UUID */))
                {
                    If ((Arg2 == Zero))
                    {
                        If ((Arg1 == One))
                        {
                            Return (Buffer (One)
                            {
                                 0x07                                             // .
                            })
                        }
                        Else
                        {
                            Return (Buffer (One)
                            {
                                 0x01                                             // .
                            })
                        }
                    }

                    If ((Arg2 == One))
                    {
                        Name (PCPN, Buffer (One)
                        {
                             0x00                                             // .
                        })
                        CreateBitField (PCPN, Zero, PCP0)
                        CreateBitField (PCPN, One, PCP1)
                        If ((DUID == Zero))
                        {
                            If ((TRE0 == One))
                            {
                                PCP0 = One
                            }

                            If ((TRE1 == One))
                            {
                                PCP1 = One
                            }
                        }
                        Else
                        {
                            If ((TRE2 == One))
                            {
                                PCP0 = One
                            }

                            If ((TRE3 == One))
                            {
                                PCP1 = One
                            }
                        }

                        Return (PCPN) /* \_SB_.PC00.TDM1._DSM.PCPN */
                    }

                    If ((Arg2 == 0x02))
                    {
                        Name (TCAP, Buffer (One)
                        {
                             0x00                                             // .
                        })
                        CreateBitField (TCAP, Zero, IMRV)
                        CreateBitField (TCAP, One, WAKE)
                        CreateBitField (TCAP, 0x02, TVDS)
                        If (ITIM)
                        {
                            IMRV = One
                        }

                        If (TIVS)
                        {
                            WAKE = One
                        }

                        If (ITVD)
                        {
                            TVDS = One
                        }

                        Return (TCAP) /* \_SB_.PC00.TDM1._DSM.TCAP */
                    }
                }
                ElseIf ((Arg0 == Buffer (0x10)
                        {
                            /* 0000 */  0x32, 0x52, 0x42, 0x1B, 0x4C, 0x15, 0x34, 0x4F,  // 2RB.L.4O
                            /* 0008 */  0x7B, 0x9E, 0xB6, 0x5C, 0x94, 0x64, 0x94, 0xA3   // {..\.d..
                        }))
                {
                    Switch (ToInteger (Arg2))
                    {
                        Case (Zero)
                        {
                            ADBG (Concatenate ("TBT DMA _DSM DUID:", ToHexString (DUID)))
                            Switch (ToInteger (Arg1))
                            {
                                Case (One)
                                {
                                    Return (Buffer (One)
                                    {
                                         0x03                                             // .
                                    })
                                }

                            }

                            ADBG ("return buffer 0x00")
                            Return (Buffer (One)
                            {
                                 0x00                                             // .
                            })
                        }
                        Case (One)
                        {
                            ADBG (Concatenate ("TBT DMA _DSM Func 1 DUID:", ToHexString (DUID)))
                            If ((DUID == One))
                            {
                                ADBG (Concatenate ("ITM1:", ToHexString (ITM1)))
                                Return (ITM1) /* External reference */
                            }
                            Else
                            {
                                ADBG (Concatenate ("ITM0:", ToHexString (ITM0)))
                                Return (ITM0) /* External reference */
                            }
                        }

                    }
                }

                Return (Buffer (One)
                {
                     0x00                                             // .
                })
            }

            Method (STCM, 2, Serialized)
            {
                ADBG (Concatenate ("STCM DUID = ", ToHexString (DUID)))
                ADBG (Concatenate ("Requested CM mode = ", Arg0))
                ADBG (Concatenate ("Pre-boot CM Info  = ", ToHexString (Arg1)))
                Local0 = (Arg1 & 0x07)
                Local1 = One
                If (((Arg1 & 0x80) != 0x80))
                {
                    ADBG ("U4CM valid bit is not set!")
                    Return (0xFF)
                }
                Else
                {
                    SCME = Local0
                    If ((Arg0 == Local0))
                    {
                        ADBG ("Pre-boot CM mode aligns with OS preferred CM mode")
                        Local1 = Zero
                    }
                }

                If ((Local1 == One))
                {
                    If ((CHCM (Arg0) == Zero))
                    {
                        ADBG (Concatenate ("Switch CM mode success, request mode = ", Arg0))
                        SCME = Arg0
                    }
                    Else
                    {
                        ADBG (Concatenate ("Switch CM mode failure, request mode = ", Arg0))
                        Return (0xFF)
                    }
                }

                ADBG (Concatenate ("SCME - ", ToHexString (SCME)))
                Return (SCME) /* \_SB_.PC00.TDM1.SCME */
            }

            Field (DPME, DWordAcc, NoLock, Preserve)
            {
                Offset (0xC8), 
                    ,   31, 
                INFR,   1, 
                Offset (0xEC), 
                TB2P,   32, 
                P2TB,   32
            }

            Method (ITMB, 1, Serialized)
            {
                ADBG (ToHexString (_DDN))
                ADBG (Concatenate ("MailBox Cmd ", ToHexString ((Arg0 >> One))))
                Local0 = (Arg0 | One)
                P2TB = Local0
                ADBG ("Command is sent")
            }

            Method (WFCC, 1, Serialized)
            {
                WTBS (Arg0)
                P2TB = Zero
                WTBC (Arg0)
            }

            Method (WTBS, 1, Serialized)
            {
                Local0 = Arg0
                ADBG (Concatenate ("Wait TB2P to be set, DUID ", ToHexString (DUID)))
                While ((Local0 > Zero))
                {
                    If ((TB2P & One))
                    {
                        ADBG (Concatenate ("TB2P is set, DUID ", ToHexString (DUID)))
                        ADBG (Concatenate ("Wait count = ", ToHexString ((Arg0 - Local0))))
                        Break
                    }

                    Local0--
                    Sleep (One)
                }

                If ((Local0 == Zero))
                {
                    ADBG (Concatenate ("Command Timeout, DUID ", ToHexString (DUID)))
                    ADBG (Concatenate ("Timeout Value ", ToHexString (Arg0)))
                }

                Return (Local0)
            }

            Method (WTBC, 1, Serialized)
            {
                Local0 = Arg0
                ADBG (Concatenate ("Wait TB2P to be clear, DUID ", ToHexString (DUID)))
                While ((Local0 > Zero))
                {
                    If (((TB2P & One) != One))
                    {
                        ADBG (Concatenate ("TB2P is clear, DUID ", ToHexString (DUID)))
                        Break
                    }

                    Local0--
                    Sleep (One)
                }

                If ((Local0 == Zero))
                {
                    ADBG (Concatenate ("Command Timeout, DUID ", ToHexString (DUID)))
                    ADBG (Concatenate ("Timeout Value ", ToHexString (Arg0)))
                }
            }

            Method (CNTP, 0, Serialized)
            {
                ADBG (Concatenate ("TCSS Send ITBT CONNECT TOPOLOGY Cmd Start, DUID -", ToHexString (DUID)))
                Local0 = Zero
                If ((DFPE == Zero))
                {
                    DMAD = 0x22
                    DFPE = One
                    While (((INFR == Zero) && (Local0 < ITFP)))
                    {
                        Sleep (One)
                        Local0++
                    }
                }

                If ((Local0 != ITFP))
                {
                    ITMB (0x3E)
                }
                Else
                {
                    ADBG ("Force Power Timeout occurred")
                }

                If ((DFPE != Zero))
                {
                    ADBG ("Unset Force Power")
                    DFPE = Zero
                }

                ADBG (Concatenate ("TCSS Send ITBT CONNECT TOPOLOGY Cmd End, DUID -", ToHexString (DUID)))
            }

            Method (CMMB, 1, Serialized)
            {
                If ((Arg0 == Zero))
                {
                    ADBG ("Set FW CM mode")
                    ITMB (0x44)
                }
                ElseIf ((Arg0 == One))
                {
                    ADBG ("Set SW CM mode")
                    ITMB (0x46)
                }

                Local0 = WTBS (0x01F4)
                If ((Local0 != Zero))
                {
                    ADBG ("Set CM mode done bit is set")
                    Local1 = Zero
                }
                Else
                {
                    ADBG ("Set CM mode command timeout")
                    Local1 = 0xFF
                }

                P2TB = Zero
                Return (Local1)
            }

            Method (CHCM, 1, Serialized)
            {
                ADBG (Concatenate ("iTBT CHCM - Switch CM mode to ", Arg0))
                If ((Arg0 > One))
                {
                    ADBG ("Invalid CM mode value")
                    Return (0xFF)
                }

                Local0 = Zero
                If ((DFPE == Zero))
                {
                    ADBG ("Set Force Power")
                    DMAD = 0x22
                    DFPE = One
                    While (((INFR == Zero) && (Local0 < ITFP)))
                    {
                        Sleep (One)
                        Local0++
                    }
                }
                Else
                {
                    ADBG ("Force Power has been set")
                }

                If ((Local0 != ITFP))
                {
                    Local1 = CMMB (Arg0)
                    If (((Arg0 == Zero) && (Local1 == Zero)))
                    {
                        ADBG ("Send Connect Topology command")
                        ITMB (0x3E)
                        Local2 = WTBS (ITCT)
                        If ((Local2 != Zero))
                        {
                            ADBG ("Connect topology command done bit is set")
                            Local1 = Zero
                        }
                        Else
                        {
                            ADBG ("Connect topology command timeout!")
                            Local1 = 0xFF
                        }

                        P2TB = Zero
                    }
                }
                Else
                {
                    ADBG ("Force Power Timeout occurred")
                    Local1 = 0xFF
                }

                If ((DFPE != Zero))
                {
                    ADBG ("Unset Force Power")
                    DFPE = Zero
                }

                ADBG (Concatenate ("iTBT change CM mode status = ", Local1))
                Return (Local1)
            }
        }

        Device (TRP0)
        {
            Method (_ADR, 0, NotSerialized)  // _ADR: Address
            {
                If ((TPA0 != Zero))
                {
                    ADBG (Concatenate ("TRP0 _ADR", ToHexString (TPA0)))
                    Return (TPA0) /* External reference */
                }
                Else
                {
                    Return (0x00070000)
                }
            }

            Name (TUID, Zero)
            Name (LTEN, Zero)
            Name (LMSL, 0x88C8)
            Name (LNSL, 0x88C8)
            Name (DCPM, Zero)
            Method (_STA, 0, NotSerialized)  // _STA: Status
            {
                ADBG (Concatenate ("TCSS RP0 _STA", ToHexString (TRE0)))
                If ((TRE0 == One))
                {
                    Return (0x0F)
                }

                Return (Zero)
            }

            Method (_INI, 0, NotSerialized)  // _INI: Initialize
            {
                LTEN = LTE0 /* External reference */
                LMSL = PSL0 /* External reference */
                LNSL = PNS0 /* External reference */
            }

            Name (PRTP, 0x08)
            Name (BCFG, Ones)
            Method (BASE, 0, NotSerialized)
            {
                If ((BCFG != Ones))
                {
                    ADBG (Concatenate ("PCIe MMIO Address ", ToHexString (BCFG)))
                    Return (BCFG) /* \_SB_.PC00.TRP0.BCFG */
                }
                Else
                {
                    Local0 = (_ADR () & 0x07)
                    Local1 = ((_ADR () >> 0x10) & 0x1F)
                    ADBG (Concatenate ("BASE of ITBT Port ", ToHexString (TUID)))
                    Local2 = ((Local0 << 0x0C) + (Local1 << 0x0F))
                    Local3 = (\_SB.PC00.GPCB () + Local2)
                    BCFG = Local3
                    ADBG (Concatenate ("PCIe MMIO Address ", ToHexString (BCFG)))
                    Return (BCFG) /* \_SB_.PC00.TRP0.BCFG */
                }
            }

            Method (GOPB, 1, NotSerialized)
            {
                Local0 = (BASE () + Arg0)
                ADBG (Concatenate ("TCSS PCIE RP register offset MMIO Address ", ToHexString (Local0)))
                Return (Local0)
            }

            OperationRegion (PXCS, SystemMemory, BASE (), 0x0C00)
            Field (PXCS, AnyAcc, NoLock, Preserve)
            {
                VDID,   32, 
                Offset (0x50), 
                L0SE,   1, 
                    ,   3, 
                LDIS,   1, 
                Offset (0x51), 
                Offset (0x52), 
                    ,   13, 
                LASX,   1, 
                Offset (0x5A), 
                ABPX,   1, 
                    ,   2, 
                PDCX,   1, 
                    ,   2, 
                PDSX,   1, 
                Offset (0x5B), 
                DLSC,   1, 
                Offset (0x60), 
                Offset (0x62), 
                PSPX,   1, 
                Offset (0xA4), 
                D3HT,   2, 
                Offset (0x328), 
                STS1,   32
            }

            OperationRegion (TPCS, SystemMemory, GOPB (0x0420), 0x04)
            Field (TPCS, AnyAcc, NoLock, Preserve)
            {
                    ,   30, 
                DPGE,   1
            }

            OperationRegion (ADCS, SystemMemory, GOPB (0x05BC), 0x04)
            Field (ADCS, AnyAcc, NoLock, Preserve)
            {
                    ,   3, 
                RPER,   1, 
                RPFE,   1
            }

            OperationRegion (CCCS, SystemMemory, GOPB (RPCC), 0x20)
            Field (CCCS, AnyAcc, NoLock, WriteAsZeros)
            {
                Offset (0x0C), 
                    ,   30, 
                HPSX,   1, 
                PMSX,   1
            }

            Field (CCCS, AnyAcc, NoLock, Preserve)
            {
                Offset (0x08), 
                    ,   30, 
                HPEX,   1, 
                PMEX,   1, 
                Offset (0x12), 
                    ,   2, 
                L23E,   1, 
                L23R,   1
            }

            Field (PXCS, AnyAcc, NoLock, Preserve)
            {
                Offset (0x68), 
                    ,   10, 
                LNRE,   1, 
                Offset (0x404), 
                LSOE,   1, 
                LNSE,   1
            }

            Name (LTRV, Package (0x04)
            {
                Zero, 
                Zero, 
                Zero, 
                Zero
            })
            Method (_DSM, 4, Serialized)  // _DSM: Device-Specific Method
            {
                If ((Arg0 == ToUUID ("e5c937d0-3553-4d7a-9117-ea4d19c3434d") /* Device Labeling Interface */))
                {
                    Switch (ToInteger (Arg2))
                    {
                        Case (Zero)
                        {
                            Name (OPTS, Buffer (0x02)
                            {
                                 0x00, 0x00                                       // ..
                            })
                            CreateBitField (OPTS, Zero, FUN0)
                            CreateBitField (OPTS, 0x05, FUN5)
                            CreateBitField (OPTS, 0x06, FUN6)
                            CreateBitField (OPTS, 0x08, FUN8)
                            CreateBitField (OPTS, 0x09, FUN9)
                            FUN0 = One
                            FUN5 = IORB /* \IORB */
                            If ((LTEN == One))
                            {
                                FUN6 = One
                            }

                            If ((Arg1 >= 0x02))
                            {
                                If (CondRefOf (ECR1))
                                {
                                    If ((ECR1 == One))
                                    {
                                        If ((Arg1 >= 0x03))
                                        {
                                            FUN8 = One
                                            FUN9 = One
                                        }
                                    }
                                }
                            }

                            ADBG (Concatenate ("TBT RP OPTS -", ToHexString (OPTS)))
                            Return (OPTS) /* \_SB_.PC00.TRP0._DSM.OPTS */
                        }
                        Case (0x05)
                        {
                            If ((Arg1 >= One))
                            {
                                Return (One)
                            }
                        }
                        Case (0x06)
                        {
                            If ((Arg1 >= 0x02))
                            {
                                LTRV [Zero] = ((LMSL >> 0x0A) & 0x07)
                                LTRV [One] = (LMSL & 0x03FF)
                                LTRV [0x02] = ((LNSL >> 0x0A) & 0x07)
                                LTRV [0x03] = (LNSL & 0x03FF)
                                Return (LTRV) /* \_SB_.PC00.TRP0.LTRV */
                            }
                        }
                        Case (0x08)
                        {
                            If (CondRefOf (ECR1))
                            {
                                If ((ECR1 == One))
                                {
                                    If ((Arg1 >= 0x03))
                                    {
                                        Return (One)
                                    }
                                }
                            }
                        }
                        Case (0x09)
                        {
                            If (CondRefOf (ECR1))
                            {
                                If ((ECR1 == One))
                                {
                                    If ((Arg1 >= 0x03))
                                    {
                                        Return (Package (0x05)
                                        {
                                            0xC350, 
                                            Ones, 
                                            Ones, 
                                            0xC350, 
                                            Ones
                                        })
                                    }
                                }
                            }
                        }

                    }
                }

                Return (Buffer (One)
                {
                     0x00                                             // .
                })
            }

            Device (PXSX)
            {
                Name (_ADR, Zero)  // _ADR: Address
                Method (_PRW, 0, NotSerialized)  // _PRW: Power Resources for Wake
                {
                    If (CondRefOf (GP1E))
                    {
                        If ((GP1E != Zero))
                        {
                            Return (GPRW (0xC7, 0x04))
                        }
                    }

                    Return (GPRW (0x69, 0x04))
                }
            }

            Method (_DSW, 3, NotSerialized)  // _DSW: Device Sleep Wake
            {
                ADBG (Concatenate ("TCSS RP _DSW TUID -", ToHexString (TUID)))
                ADBG (Concatenate ("Arg0 -", ToHexString (Arg0)))
                ADBG (Concatenate ("Arg1 -", ToHexString (Arg1)))
                \_SB.PC00.TDM0.SD3C = Arg1
                \_SB.PC00.TDM1.SD3C = Arg1
                ADBG (Concatenate ("TDM0 SD3C -", ToHexString (\_SB.PC00.TDM0.SD3C)))
                ADBG (Concatenate ("TDM1 SD3C -", ToHexString (\_SB.PC00.TDM1.SD3C)))
                C2PM (Arg0, Arg1, Arg2, DCPM)
            }

            Method (_PRW, 0, NotSerialized)  // _PRW: Power Resources for Wake
            {
                If (CondRefOf (GP1E))
                {
                    If ((GP1E != Zero))
                    {
                        Return (GPRW (0xC7, 0x04))
                    }
                }

                Return (GPRW (0x69, 0x04))
            }

            Method (LTRC, 0, Serialized)
            {
                ADBG (Concatenate ("TCSS RP LTREN check TUID -", ToHexString (TUID)))
                If ((VDID != 0xFFFFFFFF))
                {
                    ADBG (Concatenate ("Current DCTL2 LTREN -", ToHexString (LNRE)))
                    HPSX = One
                    If ((LASX == Zero))
                    {
                        LNRE = One
                        Sleep (One)
                        LNRE = Zero
                        ADBG (Concatenate ("Toggle LTREN to TUID -", ToHexString (TUID)))
                    }
                }
            }

            Method (CRPM, 0, Serialized)
            {
                If ((VDID != 0xFFFFFFFF))
                {
                    Return (D3HT) /* \_SB_.PC00.TRP0.D3HT */
                }

                Return (0xFF)
            }

            Method (HPEV, 0, Serialized)
            {
                If (((VDID != 0xFFFFFFFF) && HPSX))
                {
                    ADBG (Concatenate ("HotPlug Event Start for ITBT Port - ", ToHexString (TUID)))
                    If (((PDCX == One) && (DLSC == One)))
                    {
                        PDCX = One
                        HPSX = One
                        Notify (^, Zero) // Bus Check
                    }
                    Else
                    {
                        HPSX = One
                    }

                    ADBG (Concatenate ("HotPlug Event End for ITBT Port - ", ToHexString (TUID)))
                }
            }

            Name (STAT, One)
            Method (D3CX, 0, Serialized)
            {
                ADBG (Concatenate ("TBT RP D3ColdExit Start TUID -", ToHexString (TUID)))
                ADBG (Concatenate ("RP STAT -", ToHexString (STAT)))
                ADBG (Concatenate ("TBT RP VDID -", ToHexString (VDID)))
                ADBG (Concatenate ("VDID -", ToHexString (VDID)))
                If ((STAT == One))
                {
                    Return (Zero)
                }

                ADBG (Concatenate ("Original RPFE -", ToHexString (RPFE)))
                RPFE = Zero
                ADBG (Concatenate ("RPFE -", ToHexString (RPFE)))
                ADBG (Concatenate ("Original RPER -", ToHexString (RPER)))
                RPER = Zero
                ADBG (Concatenate ("RPER -", ToHexString (RPER)))
                L23R = One
                Local0 = Zero
                Local1 = L23R /* \_SB_.PC00.TRP0.L23R */
                While (Local1)
                {
                    If ((Local0 > 0x64))
                    {
                        Break
                    }

                    Local2 = Zero
                    While ((Local2 < 0x0A))
                    {
                        Stall (0x64)
                        Local2++
                    }

                    Local0++
                    Local1 = L23R /* \_SB_.PC00.TRP0.L23R */
                }

                ADBG (Concatenate ("L23R -", ToHexString (L23R)))
                ADBG (Concatenate ("Loop -", ToHexString (Local0)))
                STAT = One
                Local0 = Zero
                Local1 = LASX /* \_SB_.PC00.TRP0.LASX */
                If ((PLAT > Zero))
                {
                    While ((Local1 == Zero))
                    {
                        If ((Local0 > PLAT))
                        {
                            Break
                        }

                        Sleep (0x05)
                        Local0 += 0x05
                        Local1 = LASX /* \_SB_.PC00.TRP0.LASX */
                    }
                }

                ADBG (Concatenate ("LASX -", ToHexString (LASX)))
                ADBG (Concatenate ("PLAT -", ToHexString (PLAT)))
                ADBG ("TBT RP D3ColdExit End")
            }

            Method (D3CE, 0, Serialized)
            {
                ADBG (Concatenate ("TBT RP D3ColdEntry Start TUID -", ToHexString (TUID)))
                ADBG (Concatenate ("RP STAT -", ToHexString (STAT)))
                ADBG (Concatenate ("TBT RP VDID -", ToHexString (VDID)))
                If ((STAT == Zero))
                {
                    Return (Zero)
                }

                If (((LNSE == One) || (LSOE == One)))
                {
                    If (((LASX == One) && (LNRE == Zero)))
                    {
                        LNRE = One
                        Sleep (One)
                        LNRE = Zero
                        ADBG ("Toggle LNRE -")
                    }
                }

                L23E = One
                Local0 = Zero
                Local1 = L23E /* \_SB_.PC00.TRP0.L23E */
                While (Local1)
                {
                    If ((Local0 > 0x64))
                    {
                        Break
                    }

                    Local2 = Zero
                    While ((Local2 < 0x0A))
                    {
                        Stall (0x64)
                        Local2++
                    }

                    Local0++
                    Local1 = L23E /* \_SB_.PC00.TRP0.L23E */
                }

                ADBG (Concatenate ("L23E -", ToHexString (L23E)))
                ADBG (Concatenate ("Loop -", ToHexString (Local0)))
                STAT = Zero
                ADBG (Concatenate ("Original RPFE -", ToHexString (RPFE)))
                RPFE = One
                ADBG (Concatenate ("RPFE -", ToHexString (RPFE)))
                ADBG (Concatenate ("Original RPER -", ToHexString (RPER)))
                RPER = One
                ADBG (Concatenate ("RPER -", ToHexString (RPER)))
                ADBG (Concatenate ("TBT RP D3ColdEntry End TUID -", ToHexString (TUID)))
            }

            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
            {
                ADBG (Concatenate ("TBT _PS0 Start RP ", ToHexString (TUID)))
                ADBG (Concatenate ("TBT RP VDID -", ToHexString (VDID)))
                HPEV ()
                HPME ()
                If ((PMEX == One))
                {
                    ADBG ("Disable PME SCI")
                    PMEX = Zero
                }

                Sleep (PLAT)
                If (((TUID == Zero) || (TUID == One)))
                {
                    If ((\_SB.PC00.TDM0.WACT == One))
                    {
                        \_SB.PC00.TDM0.WACT = 0x02
                        \_SB.PC00.TDM0.WFCC (ITCT)
                        \_SB.PC00.TDM0.WACT = Zero
                    }
                    ElseIf ((\_SB.PC00.TDM0.WACT == 0x02))
                    {
                        ADBG ("Wait until other _PS0 get response")
                        While ((\_SB.PC00.TDM0.WACT != Zero))
                        {
                            Sleep (0x05)
                        }

                        ADBG ("Other _PS0 got response")
                    }
                }
                ElseIf ((\_SB.PC00.TDM1.WACT == One))
                {
                    \_SB.PC00.TDM1.WACT = 0x02
                    \_SB.PC00.TDM1.WFCC (ITCT)
                    \_SB.PC00.TDM1.WACT = Zero
                }
                ElseIf ((\_SB.PC00.TDM1.WACT == 0x02))
                {
                    ADBG ("Wait until other _PS0 get response")
                    While ((\_SB.PC00.TDM1.WACT != Zero))
                    {
                        Sleep (0x05)
                    }

                    ADBG ("Other _PS0 got response")
                }

                ADBG (Concatenate ("TBT _PS0 End RP ", ToHexString (TUID)))
            }

            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
            {
                ADBG (Concatenate ("TBT _PS3 Start RP ", ToHexString (TUID)))
                ADBG (Concatenate ("TBT RP VDID -", ToHexString (VDID)))
                CHKH ()
                If ((HPEX == Zero))
                {
                    ADBG ("Enable Hot Plug SCI")
                    HPEX = One
                    HPEV ()
                }

                If ((PMEX == Zero))
                {
                    ADBG ("Enable PME SCI")
                    PMEX = One
                    HPME ()
                }

                ADBG (Concatenate ("TBT _PS3 End RP ", ToHexString (TUID)))
            }

            Method (CHKH, 0, NotSerialized)
            {
                If ((PDCX == One))
                {
                    ADBG ("PDC get set before enable HotPlug SCI")
                    If ((DLSC == Zero))
                    {
                        ADBG ("Clear PDC since it is not a real hotplug")
                        PDCX = One
                    }
                }
            }

            Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
            {
                ADBG ("TBT RP _DSD")
                ADBG (Concatenate ("TUID = ", ToHexString (TUID)))
                If (CondRefOf (U4CM))
                {
                    If (((U4CM & 0x07) == Zero))
                    {
                        Return (Package (0x04)
                        {
                            ToUUID ("6211e2c0-58a3-4af3-90e1-927a4e0c55a4") /* Unknown UUID */, 
                            Package (0x01)
                            {
                                Package (0x02)
                                {
                                    "HotPlugSupportInD3", 
                                    One
                                }
                            }, 

                            ToUUID ("efcc06cc-73ac-4bc3-bff0-76143807c389") /* Unknown UUID */, 
                            Package (0x02)
                            {
                                Package (0x02)
                                {
                                    "ExternalFacingPort", 
                                    One
                                }, 

                                Package (0x02)
                                {
                                    "UID", 
                                    TUID
                                }
                            }
                        })
                    }
                }

                If ((((TUID == Zero) || (TUID == One)) && (DME0 == One)))
                {
                    Return (Package (0x06)
                    {
                        ToUUID ("6211e2c0-58a3-4af3-90e1-927a4e0c55a4") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "HotPlugSupportInD3", 
                                One
                            }
                        }, 

                        ToUUID ("efcc06cc-73ac-4bc3-bff0-76143807c389") /* Unknown UUID */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "ExternalFacingPort", 
                                One
                            }, 

                            Package (0x02)
                            {
                                "UID", 
                                TUID
                            }
                        }, 

                        ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "usb4-host-interface", 
                                \_SB.PC00.TDM0
                            }, 

                            Package (0x02)
                            {
                                "usb4-port-number", 
                                TUID
                            }
                        }
                    })
                }
                ElseIf ((((TUID == 0x02) || (TUID == 0x03)) && (DME1 == 
                    One)))
                {
                    Return (Package (0x06)
                    {
                        ToUUID ("6211e2c0-58a3-4af3-90e1-927a4e0c55a4") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "HotPlugSupportInD3", 
                                One
                            }
                        }, 

                        ToUUID ("efcc06cc-73ac-4bc3-bff0-76143807c389") /* Unknown UUID */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "ExternalFacingPort", 
                                One
                            }, 

                            Package (0x02)
                            {
                                "UID", 
                                TUID
                            }
                        }, 

                        ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "usb4-host-interface", 
                                \_SB.PC00.TDM1
                            }, 

                            Package (0x02)
                            {
                                "usb4-port-number", 
                                TUID
                            }
                        }
                    })
                }
                Else
                {
                    Return (Package (0x04)
                    {
                        ToUUID ("6211e2c0-58a3-4af3-90e1-927a4e0c55a4") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "HotPlugSupportInD3", 
                                One
                            }
                        }, 

                        ToUUID ("efcc06cc-73ac-4bc3-bff0-76143807c389") /* Unknown UUID */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "ExternalFacingPort", 
                                One
                            }, 

                            Package (0x02)
                            {
                                "UID", 
                                TUID
                            }
                        }
                    })
                }
            }

            Method (_S0W, 0, NotSerialized)  // _S0W: S0 Device Wake State
            {
                If (ITRT)
                {
                    Return (0x04)
                }
                Else
                {
                    Return (0x03)
                }
            }

            If (ITRT)
            {
                Method (_PR0, 0, NotSerialized)  // _PR0: Power Resources for D0
                {
                    If (TRTD)
                    {
                        If (((\_SB.PC00.TDM0.SCME == One) || (\_SB.PC00.TDM1.SCME == One)))
                        {
                            Return (Package (0x01)
                            {
                                \_SB.PC00.D3C
                            })
                        }

                        If (((TUID == Zero) || (TUID == One)))
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT0
                            })
                        }
                        Else
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT1
                            })
                        }
                    }

                    If (((TUID == Zero) || (TUID == One)))
                    {
                        Return (Package (0x01)
                        {
                            \_SB.PC00.TBT0
                        })
                    }

                    Return (Package (0x01)
                    {
                        \_SB.PC00.TBT1
                    })
                }

                Method (_PR3, 0, NotSerialized)  // _PR3: Power Resources for D3hot
                {
                    If (TRTD)
                    {
                        If (((\_SB.PC00.TDM0.SCME == One) || (\_SB.PC00.TDM1.SCME == One)))
                        {
                            Return (Package (0x01)
                            {
                                \_SB.PC00.D3C
                            })
                        }

                        If (((TUID == Zero) || (TUID == One)))
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT0
                            })
                        }
                        Else
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT1
                            })
                        }
                    }

                    If (((TUID == Zero) || (TUID == One)))
                    {
                        Return (Package (0x01)
                        {
                            \_SB.PC00.TBT0
                        })
                    }

                    Return (Package (0x01)
                    {
                        \_SB.PC00.TBT1
                    })
                }
            }

            Method (HPME, 0, Serialized)
            {
                ADBG ("TBT HPME")
                If (((VDID != 0xFFFFFFFF) && (PMSX == One)))
                {
                    ADBG ("TBT RP exists")
                    Notify (PXSX, 0x02) // Device Wake
                    PMSX = One
                    PSPX = One
                    Return (One)
                }

                Return (Zero)
            }

            Name (AR01, Package (0x04)
            {
                Package (0x04)
                {
                    0xFFFF, 
                    Zero, 
                    Zero, 
                    0x10
                }, 

                Package (0x04)
                {
                    0xFFFF, 
                    One, 
                    Zero, 
                    0x11
                }, 

                Package (0x04)
                {
                    0xFFFF, 
                    0x02, 
                    Zero, 
                    0x12
                }, 

                Package (0x04)
                {
                    0xFFFF, 
                    0x03, 
                    Zero, 
                    0x13
                }
            })
            Method (_PRT, 0, NotSerialized)  // _PRT: PCI Routing Table
            {
                Return (AR01) /* \_SB_.PC00.TRP0.AR01 */
            }
        }

        Device (TRP1)
        {
            Method (_ADR, 0, NotSerialized)  // _ADR: Address
            {
                If ((TPA1 != Zero))
                {
                    ADBG (Concatenate ("TRP1 _ADR", ToHexString (TPA1)))
                    Return (TPA1) /* External reference */
                }
                Else
                {
                    Return (0x00070001)
                }
            }

            Name (TUID, One)
            Name (LTEN, Zero)
            Name (LMSL, 0x88C8)
            Name (LNSL, 0x88C8)
            Name (DCPM, One)
            Method (_STA, 0, NotSerialized)  // _STA: Status
            {
                ADBG (Concatenate ("TCSS RP1 _STA", ToHexString (TRE1)))
                If ((TRE1 == One))
                {
                    Return (0x0F)
                }

                Return (Zero)
            }

            Method (_INI, 0, NotSerialized)  // _INI: Initialize
            {
                LTEN = LTE1 /* External reference */
                LMSL = PSL1 /* External reference */
                LNSL = PNS1 /* External reference */
            }

            Name (PRTP, 0x08)
            Name (BCFG, Ones)
            Method (BASE, 0, NotSerialized)
            {
                If ((BCFG != Ones))
                {
                    ADBG (Concatenate ("PCIe MMIO Address ", ToHexString (BCFG)))
                    Return (BCFG) /* \_SB_.PC00.TRP1.BCFG */
                }
                Else
                {
                    Local0 = (_ADR () & 0x07)
                    Local1 = ((_ADR () >> 0x10) & 0x1F)
                    ADBG (Concatenate ("BASE of ITBT Port ", ToHexString (TUID)))
                    Local2 = ((Local0 << 0x0C) + (Local1 << 0x0F))
                    Local3 = (\_SB.PC00.GPCB () + Local2)
                    BCFG = Local3
                    ADBG (Concatenate ("PCIe MMIO Address ", ToHexString (BCFG)))
                    Return (BCFG) /* \_SB_.PC00.TRP1.BCFG */
                }
            }

            Method (GOPB, 1, NotSerialized)
            {
                Local0 = (BASE () + Arg0)
                ADBG (Concatenate ("TCSS PCIE RP register offset MMIO Address ", ToHexString (Local0)))
                Return (Local0)
            }

            OperationRegion (PXCS, SystemMemory, BASE (), 0x0C00)
            Field (PXCS, AnyAcc, NoLock, Preserve)
            {
                VDID,   32, 
                Offset (0x50), 
                L0SE,   1, 
                    ,   3, 
                LDIS,   1, 
                Offset (0x51), 
                Offset (0x52), 
                    ,   13, 
                LASX,   1, 
                Offset (0x5A), 
                ABPX,   1, 
                    ,   2, 
                PDCX,   1, 
                    ,   2, 
                PDSX,   1, 
                Offset (0x5B), 
                DLSC,   1, 
                Offset (0x60), 
                Offset (0x62), 
                PSPX,   1, 
                Offset (0xA4), 
                D3HT,   2, 
                Offset (0x328), 
                STS1,   32
            }

            OperationRegion (TPCS, SystemMemory, GOPB (0x0420), 0x04)
            Field (TPCS, AnyAcc, NoLock, Preserve)
            {
                    ,   30, 
                DPGE,   1
            }

            OperationRegion (ADCS, SystemMemory, GOPB (0x05BC), 0x04)
            Field (ADCS, AnyAcc, NoLock, Preserve)
            {
                    ,   3, 
                RPER,   1, 
                RPFE,   1
            }

            OperationRegion (CCCS, SystemMemory, GOPB (RPCC), 0x20)
            Field (CCCS, AnyAcc, NoLock, WriteAsZeros)
            {
                Offset (0x0C), 
                    ,   30, 
                HPSX,   1, 
                PMSX,   1
            }

            Field (CCCS, AnyAcc, NoLock, Preserve)
            {
                Offset (0x08), 
                    ,   30, 
                HPEX,   1, 
                PMEX,   1, 
                Offset (0x12), 
                    ,   2, 
                L23E,   1, 
                L23R,   1
            }

            Field (PXCS, AnyAcc, NoLock, Preserve)
            {
                Offset (0x68), 
                    ,   10, 
                LNRE,   1, 
                Offset (0x404), 
                LSOE,   1, 
                LNSE,   1
            }

            Name (LTRV, Package (0x04)
            {
                Zero, 
                Zero, 
                Zero, 
                Zero
            })
            Method (_DSM, 4, Serialized)  // _DSM: Device-Specific Method
            {
                If ((Arg0 == ToUUID ("e5c937d0-3553-4d7a-9117-ea4d19c3434d") /* Device Labeling Interface */))
                {
                    Switch (ToInteger (Arg2))
                    {
                        Case (Zero)
                        {
                            Name (OPTS, Buffer (0x02)
                            {
                                 0x00, 0x00                                       // ..
                            })
                            CreateBitField (OPTS, Zero, FUN0)
                            CreateBitField (OPTS, 0x05, FUN5)
                            CreateBitField (OPTS, 0x06, FUN6)
                            CreateBitField (OPTS, 0x08, FUN8)
                            CreateBitField (OPTS, 0x09, FUN9)
                            FUN0 = One
                            FUN5 = IORB /* \IORB */
                            If ((LTEN == One))
                            {
                                FUN6 = One
                            }

                            If ((Arg1 >= 0x02))
                            {
                                If (CondRefOf (ECR1))
                                {
                                    If ((ECR1 == One))
                                    {
                                        If ((Arg1 >= 0x03))
                                        {
                                            FUN8 = One
                                            FUN9 = One
                                        }
                                    }
                                }
                            }

                            ADBG (Concatenate ("TBT RP OPTS -", ToHexString (OPTS)))
                            Return (OPTS) /* \_SB_.PC00.TRP1._DSM.OPTS */
                        }
                        Case (0x05)
                        {
                            If ((Arg1 >= One))
                            {
                                Return (One)
                            }
                        }
                        Case (0x06)
                        {
                            If ((Arg1 >= 0x02))
                            {
                                LTRV [Zero] = ((LMSL >> 0x0A) & 0x07)
                                LTRV [One] = (LMSL & 0x03FF)
                                LTRV [0x02] = ((LNSL >> 0x0A) & 0x07)
                                LTRV [0x03] = (LNSL & 0x03FF)
                                Return (LTRV) /* \_SB_.PC00.TRP1.LTRV */
                            }
                        }
                        Case (0x08)
                        {
                            If (CondRefOf (ECR1))
                            {
                                If ((ECR1 == One))
                                {
                                    If ((Arg1 >= 0x03))
                                    {
                                        Return (One)
                                    }
                                }
                            }
                        }
                        Case (0x09)
                        {
                            If (CondRefOf (ECR1))
                            {
                                If ((ECR1 == One))
                                {
                                    If ((Arg1 >= 0x03))
                                    {
                                        Return (Package (0x05)
                                        {
                                            0xC350, 
                                            Ones, 
                                            Ones, 
                                            0xC350, 
                                            Ones
                                        })
                                    }
                                }
                            }
                        }

                    }
                }

                Return (Buffer (One)
                {
                     0x00                                             // .
                })
            }

            Device (PXSX)
            {
                Name (_ADR, Zero)  // _ADR: Address
                Method (_PRW, 0, NotSerialized)  // _PRW: Power Resources for Wake
                {
                    If (CondRefOf (GP1E))
                    {
                        If ((GP1E != Zero))
                        {
                            Return (GPRW (0xC7, 0x04))
                        }
                    }

                    Return (GPRW (0x69, 0x04))
                }
            }

            Method (_DSW, 3, NotSerialized)  // _DSW: Device Sleep Wake
            {
                ADBG (Concatenate ("TCSS RP _DSW TUID -", ToHexString (TUID)))
                ADBG (Concatenate ("Arg0 -", ToHexString (Arg0)))
                ADBG (Concatenate ("Arg1 -", ToHexString (Arg1)))
                \_SB.PC00.TDM0.SD3C = Arg1
                \_SB.PC00.TDM1.SD3C = Arg1
                ADBG (Concatenate ("TDM0 SD3C -", ToHexString (\_SB.PC00.TDM0.SD3C)))
                ADBG (Concatenate ("TDM1 SD3C -", ToHexString (\_SB.PC00.TDM1.SD3C)))
                C2PM (Arg0, Arg1, Arg2, DCPM)
            }

            Method (_PRW, 0, NotSerialized)  // _PRW: Power Resources for Wake
            {
                If (CondRefOf (GP1E))
                {
                    If ((GP1E != Zero))
                    {
                        Return (GPRW (0xC7, 0x04))
                    }
                }

                Return (GPRW (0x69, 0x04))
            }

            Method (LTRC, 0, Serialized)
            {
                ADBG (Concatenate ("TCSS RP LTREN check TUID -", ToHexString (TUID)))
                If ((VDID != 0xFFFFFFFF))
                {
                    ADBG (Concatenate ("Current DCTL2 LTREN -", ToHexString (LNRE)))
                    HPSX = One
                    If ((LASX == Zero))
                    {
                        LNRE = One
                        Sleep (One)
                        LNRE = Zero
                        ADBG (Concatenate ("Toggle LTREN to TUID -", ToHexString (TUID)))
                    }
                }
            }

            Method (CRPM, 0, Serialized)
            {
                If ((VDID != 0xFFFFFFFF))
                {
                    Return (D3HT) /* \_SB_.PC00.TRP1.D3HT */
                }

                Return (0xFF)
            }

            Method (HPEV, 0, Serialized)
            {
                If (((VDID != 0xFFFFFFFF) && HPSX))
                {
                    ADBG (Concatenate ("HotPlug Event Start for ITBT Port - ", ToHexString (TUID)))
                    If (((PDCX == One) && (DLSC == One)))
                    {
                        PDCX = One
                        HPSX = One
                        Notify (^, Zero) // Bus Check
                    }
                    Else
                    {
                        HPSX = One
                    }

                    ADBG (Concatenate ("HotPlug Event End for ITBT Port - ", ToHexString (TUID)))
                }
            }

            Name (STAT, One)
            Method (D3CX, 0, Serialized)
            {
                ADBG (Concatenate ("TBT RP D3ColdExit Start TUID -", ToHexString (TUID)))
                ADBG (Concatenate ("RP STAT -", ToHexString (STAT)))
                ADBG (Concatenate ("TBT RP VDID -", ToHexString (VDID)))
                ADBG (Concatenate ("VDID -", ToHexString (VDID)))
                If ((STAT == One))
                {
                    Return (Zero)
                }

                ADBG (Concatenate ("Original RPFE -", ToHexString (RPFE)))
                RPFE = Zero
                ADBG (Concatenate ("RPFE -", ToHexString (RPFE)))
                ADBG (Concatenate ("Original RPER -", ToHexString (RPER)))
                RPER = Zero
                ADBG (Concatenate ("RPER -", ToHexString (RPER)))
                L23R = One
                Local0 = Zero
                Local1 = L23R /* \_SB_.PC00.TRP1.L23R */
                While (Local1)
                {
                    If ((Local0 > 0x64))
                    {
                        Break
                    }

                    Local2 = Zero
                    While ((Local2 < 0x0A))
                    {
                        Stall (0x64)
                        Local2++
                    }

                    Local0++
                    Local1 = L23R /* \_SB_.PC00.TRP1.L23R */
                }

                ADBG (Concatenate ("L23R -", ToHexString (L23R)))
                ADBG (Concatenate ("Loop -", ToHexString (Local0)))
                STAT = One
                Local0 = Zero
                Local1 = LASX /* \_SB_.PC00.TRP1.LASX */
                If ((PLAT > Zero))
                {
                    While ((Local1 == Zero))
                    {
                        If ((Local0 > PLAT))
                        {
                            Break
                        }

                        Sleep (0x05)
                        Local0 += 0x05
                        Local1 = LASX /* \_SB_.PC00.TRP1.LASX */
                    }
                }

                ADBG (Concatenate ("LASX -", ToHexString (LASX)))
                ADBG (Concatenate ("PLAT -", ToHexString (PLAT)))
                ADBG ("TBT RP D3ColdExit End")
            }

            Method (D3CE, 0, Serialized)
            {
                ADBG (Concatenate ("TBT RP D3ColdEntry Start TUID -", ToHexString (TUID)))
                ADBG (Concatenate ("RP STAT -", ToHexString (STAT)))
                ADBG (Concatenate ("TBT RP VDID -", ToHexString (VDID)))
                If ((STAT == Zero))
                {
                    Return (Zero)
                }

                If (((LNSE == One) || (LSOE == One)))
                {
                    If (((LASX == One) && (LNRE == Zero)))
                    {
                        LNRE = One
                        Sleep (One)
                        LNRE = Zero
                        ADBG ("Toggle LNRE -")
                    }
                }

                L23E = One
                Local0 = Zero
                Local1 = L23E /* \_SB_.PC00.TRP1.L23E */
                While (Local1)
                {
                    If ((Local0 > 0x64))
                    {
                        Break
                    }

                    Local2 = Zero
                    While ((Local2 < 0x0A))
                    {
                        Stall (0x64)
                        Local2++
                    }

                    Local0++
                    Local1 = L23E /* \_SB_.PC00.TRP1.L23E */
                }

                ADBG (Concatenate ("L23E -", ToHexString (L23E)))
                ADBG (Concatenate ("Loop -", ToHexString (Local0)))
                STAT = Zero
                ADBG (Concatenate ("Original RPFE -", ToHexString (RPFE)))
                RPFE = One
                ADBG (Concatenate ("RPFE -", ToHexString (RPFE)))
                ADBG (Concatenate ("Original RPER -", ToHexString (RPER)))
                RPER = One
                ADBG (Concatenate ("RPER -", ToHexString (RPER)))
                ADBG (Concatenate ("TBT RP D3ColdEntry End TUID -", ToHexString (TUID)))
            }

            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
            {
                ADBG (Concatenate ("TBT _PS0 Start RP ", ToHexString (TUID)))
                ADBG (Concatenate ("TBT RP VDID -", ToHexString (VDID)))
                HPEV ()
                HPME ()
                If ((PMEX == One))
                {
                    ADBG ("Disable PME SCI")
                    PMEX = Zero
                }

                Sleep (PLAT)
                If (((TUID == Zero) || (TUID == One)))
                {
                    If ((\_SB.PC00.TDM0.WACT == One))
                    {
                        \_SB.PC00.TDM0.WACT = 0x02
                        \_SB.PC00.TDM0.WFCC (ITCT)
                        \_SB.PC00.TDM0.WACT = Zero
                    }
                    ElseIf ((\_SB.PC00.TDM0.WACT == 0x02))
                    {
                        ADBG ("Wait until other _PS0 get response")
                        While ((\_SB.PC00.TDM0.WACT != Zero))
                        {
                            Sleep (0x05)
                        }

                        ADBG ("Other _PS0 got response")
                    }
                }
                ElseIf ((\_SB.PC00.TDM1.WACT == One))
                {
                    \_SB.PC00.TDM1.WACT = 0x02
                    \_SB.PC00.TDM1.WFCC (ITCT)
                    \_SB.PC00.TDM1.WACT = Zero
                }
                ElseIf ((\_SB.PC00.TDM1.WACT == 0x02))
                {
                    ADBG ("Wait until other _PS0 get response")
                    While ((\_SB.PC00.TDM1.WACT != Zero))
                    {
                        Sleep (0x05)
                    }

                    ADBG ("Other _PS0 got response")
                }

                ADBG (Concatenate ("TBT _PS0 End RP ", ToHexString (TUID)))
            }

            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
            {
                ADBG (Concatenate ("TBT _PS3 Start RP ", ToHexString (TUID)))
                ADBG (Concatenate ("TBT RP VDID -", ToHexString (VDID)))
                CHKH ()
                If ((HPEX == Zero))
                {
                    ADBG ("Enable Hot Plug SCI")
                    HPEX = One
                    HPEV ()
                }

                If ((PMEX == Zero))
                {
                    ADBG ("Enable PME SCI")
                    PMEX = One
                    HPME ()
                }

                ADBG (Concatenate ("TBT _PS3 End RP ", ToHexString (TUID)))
            }

            Method (CHKH, 0, NotSerialized)
            {
                If ((PDCX == One))
                {
                    ADBG ("PDC get set before enable HotPlug SCI")
                    If ((DLSC == Zero))
                    {
                        ADBG ("Clear PDC since it is not a real hotplug")
                        PDCX = One
                    }
                }
            }

            Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
            {
                ADBG ("TBT RP _DSD")
                ADBG (Concatenate ("TUID = ", ToHexString (TUID)))
                If (CondRefOf (U4CM))
                {
                    If (((U4CM & 0x07) == Zero))
                    {
                        Return (Package (0x04)
                        {
                            ToUUID ("6211e2c0-58a3-4af3-90e1-927a4e0c55a4") /* Unknown UUID */, 
                            Package (0x01)
                            {
                                Package (0x02)
                                {
                                    "HotPlugSupportInD3", 
                                    One
                                }
                            }, 

                            ToUUID ("efcc06cc-73ac-4bc3-bff0-76143807c389") /* Unknown UUID */, 
                            Package (0x02)
                            {
                                Package (0x02)
                                {
                                    "ExternalFacingPort", 
                                    One
                                }, 

                                Package (0x02)
                                {
                                    "UID", 
                                    TUID
                                }
                            }
                        })
                    }
                }

                If ((((TUID == Zero) || (TUID == One)) && (DME0 == One)))
                {
                    Return (Package (0x06)
                    {
                        ToUUID ("6211e2c0-58a3-4af3-90e1-927a4e0c55a4") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "HotPlugSupportInD3", 
                                One
                            }
                        }, 

                        ToUUID ("efcc06cc-73ac-4bc3-bff0-76143807c389") /* Unknown UUID */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "ExternalFacingPort", 
                                One
                            }, 

                            Package (0x02)
                            {
                                "UID", 
                                TUID
                            }
                        }, 

                        ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "usb4-host-interface", 
                                \_SB.PC00.TDM0
                            }, 

                            Package (0x02)
                            {
                                "usb4-port-number", 
                                TUID
                            }
                        }
                    })
                }
                ElseIf ((((TUID == 0x02) || (TUID == 0x03)) && (DME1 == 
                    One)))
                {
                    Return (Package (0x06)
                    {
                        ToUUID ("6211e2c0-58a3-4af3-90e1-927a4e0c55a4") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "HotPlugSupportInD3", 
                                One
                            }
                        }, 

                        ToUUID ("efcc06cc-73ac-4bc3-bff0-76143807c389") /* Unknown UUID */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "ExternalFacingPort", 
                                One
                            }, 

                            Package (0x02)
                            {
                                "UID", 
                                TUID
                            }
                        }, 

                        ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "usb4-host-interface", 
                                \_SB.PC00.TDM1
                            }, 

                            Package (0x02)
                            {
                                "usb4-port-number", 
                                TUID
                            }
                        }
                    })
                }
                Else
                {
                    Return (Package (0x04)
                    {
                        ToUUID ("6211e2c0-58a3-4af3-90e1-927a4e0c55a4") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "HotPlugSupportInD3", 
                                One
                            }
                        }, 

                        ToUUID ("efcc06cc-73ac-4bc3-bff0-76143807c389") /* Unknown UUID */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "ExternalFacingPort", 
                                One
                            }, 

                            Package (0x02)
                            {
                                "UID", 
                                TUID
                            }
                        }
                    })
                }
            }

            Method (_S0W, 0, NotSerialized)  // _S0W: S0 Device Wake State
            {
                If (ITRT)
                {
                    Return (0x04)
                }
                Else
                {
                    Return (0x03)
                }
            }

            If (ITRT)
            {
                Method (_PR0, 0, NotSerialized)  // _PR0: Power Resources for D0
                {
                    If (TRTD)
                    {
                        If (((\_SB.PC00.TDM0.SCME == One) || (\_SB.PC00.TDM1.SCME == One)))
                        {
                            Return (Package (0x01)
                            {
                                \_SB.PC00.D3C
                            })
                        }

                        If (((TUID == Zero) || (TUID == One)))
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT0
                            })
                        }
                        Else
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT1
                            })
                        }
                    }

                    If (((TUID == Zero) || (TUID == One)))
                    {
                        Return (Package (0x01)
                        {
                            \_SB.PC00.TBT0
                        })
                    }

                    Return (Package (0x01)
                    {
                        \_SB.PC00.TBT1
                    })
                }

                Method (_PR3, 0, NotSerialized)  // _PR3: Power Resources for D3hot
                {
                    If (TRTD)
                    {
                        If (((\_SB.PC00.TDM0.SCME == One) || (\_SB.PC00.TDM1.SCME == One)))
                        {
                            Return (Package (0x01)
                            {
                                \_SB.PC00.D3C
                            })
                        }

                        If (((TUID == Zero) || (TUID == One)))
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT0
                            })
                        }
                        Else
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT1
                            })
                        }
                    }

                    If (((TUID == Zero) || (TUID == One)))
                    {
                        Return (Package (0x01)
                        {
                            \_SB.PC00.TBT0
                        })
                    }

                    Return (Package (0x01)
                    {
                        \_SB.PC00.TBT1
                    })
                }
            }

            Method (HPME, 0, Serialized)
            {
                ADBG ("TBT HPME")
                If (((VDID != 0xFFFFFFFF) && (PMSX == One)))
                {
                    ADBG ("TBT RP exists")
                    Notify (PXSX, 0x02) // Device Wake
                    PMSX = One
                    PSPX = One
                    Return (One)
                }

                Return (Zero)
            }

            Name (AR01, Package (0x04)
            {
                Package (0x04)
                {
                    0xFFFF, 
                    Zero, 
                    Zero, 
                    0x10
                }, 

                Package (0x04)
                {
                    0xFFFF, 
                    One, 
                    Zero, 
                    0x11
                }, 

                Package (0x04)
                {
                    0xFFFF, 
                    0x02, 
                    Zero, 
                    0x12
                }, 

                Package (0x04)
                {
                    0xFFFF, 
                    0x03, 
                    Zero, 
                    0x13
                }
            })
            Method (_PRT, 0, NotSerialized)  // _PRT: PCI Routing Table
            {
                Return (AR01) /* \_SB_.PC00.TRP1.AR01 */
            }
        }

        Device (TRP2)
        {
            Method (_ADR, 0, NotSerialized)  // _ADR: Address
            {
                If ((TPA2 != Zero))
                {
                    ADBG (Concatenate ("TRP2 _ADR", ToHexString (TPA2)))
                    Return (TPA2) /* External reference */
                }
                Else
                {
                    Return (0x00070002)
                }
            }

            Name (TUID, 0x02)
            Name (LTEN, Zero)
            Name (LMSL, 0x88C8)
            Name (LNSL, 0x88C8)
            Name (DCPM, 0x02)
            Method (_STA, 0, NotSerialized)  // _STA: Status
            {
                ADBG (Concatenate ("TCSS RP2 _STA", ToHexString (TRE2)))
                If ((TRE2 == One))
                {
                    Return (0x0F)
                }

                Return (Zero)
            }

            Method (_INI, 0, NotSerialized)  // _INI: Initialize
            {
                LTEN = LTE2 /* External reference */
                LMSL = PSL2 /* External reference */
                LNSL = PNS2 /* External reference */
            }

            Name (PRTP, 0x08)
            Name (BCFG, Ones)
            Method (BASE, 0, NotSerialized)
            {
                If ((BCFG != Ones))
                {
                    ADBG (Concatenate ("PCIe MMIO Address ", ToHexString (BCFG)))
                    Return (BCFG) /* \_SB_.PC00.TRP2.BCFG */
                }
                Else
                {
                    Local0 = (_ADR () & 0x07)
                    Local1 = ((_ADR () >> 0x10) & 0x1F)
                    ADBG (Concatenate ("BASE of ITBT Port ", ToHexString (TUID)))
                    Local2 = ((Local0 << 0x0C) + (Local1 << 0x0F))
                    Local3 = (\_SB.PC00.GPCB () + Local2)
                    BCFG = Local3
                    ADBG (Concatenate ("PCIe MMIO Address ", ToHexString (BCFG)))
                    Return (BCFG) /* \_SB_.PC00.TRP2.BCFG */
                }
            }

            Method (GOPB, 1, NotSerialized)
            {
                Local0 = (BASE () + Arg0)
                ADBG (Concatenate ("TCSS PCIE RP register offset MMIO Address ", ToHexString (Local0)))
                Return (Local0)
            }

            OperationRegion (PXCS, SystemMemory, BASE (), 0x0C00)
            Field (PXCS, AnyAcc, NoLock, Preserve)
            {
                VDID,   32, 
                Offset (0x50), 
                L0SE,   1, 
                    ,   3, 
                LDIS,   1, 
                Offset (0x51), 
                Offset (0x52), 
                    ,   13, 
                LASX,   1, 
                Offset (0x5A), 
                ABPX,   1, 
                    ,   2, 
                PDCX,   1, 
                    ,   2, 
                PDSX,   1, 
                Offset (0x5B), 
                DLSC,   1, 
                Offset (0x60), 
                Offset (0x62), 
                PSPX,   1, 
                Offset (0xA4), 
                D3HT,   2, 
                Offset (0x328), 
                STS1,   32
            }

            OperationRegion (TPCS, SystemMemory, GOPB (0x0420), 0x04)
            Field (TPCS, AnyAcc, NoLock, Preserve)
            {
                    ,   30, 
                DPGE,   1
            }

            OperationRegion (ADCS, SystemMemory, GOPB (0x05BC), 0x04)
            Field (ADCS, AnyAcc, NoLock, Preserve)
            {
                    ,   3, 
                RPER,   1, 
                RPFE,   1
            }

            OperationRegion (CCCS, SystemMemory, GOPB (RPCC), 0x20)
            Field (CCCS, AnyAcc, NoLock, WriteAsZeros)
            {
                Offset (0x0C), 
                    ,   30, 
                HPSX,   1, 
                PMSX,   1
            }

            Field (CCCS, AnyAcc, NoLock, Preserve)
            {
                Offset (0x08), 
                    ,   30, 
                HPEX,   1, 
                PMEX,   1, 
                Offset (0x12), 
                    ,   2, 
                L23E,   1, 
                L23R,   1
            }

            Field (PXCS, AnyAcc, NoLock, Preserve)
            {
                Offset (0x68), 
                    ,   10, 
                LNRE,   1, 
                Offset (0x404), 
                LSOE,   1, 
                LNSE,   1
            }

            Name (LTRV, Package (0x04)
            {
                Zero, 
                Zero, 
                Zero, 
                Zero
            })
            Method (_DSM, 4, Serialized)  // _DSM: Device-Specific Method
            {
                If ((Arg0 == ToUUID ("e5c937d0-3553-4d7a-9117-ea4d19c3434d") /* Device Labeling Interface */))
                {
                    Switch (ToInteger (Arg2))
                    {
                        Case (Zero)
                        {
                            Name (OPTS, Buffer (0x02)
                            {
                                 0x00, 0x00                                       // ..
                            })
                            CreateBitField (OPTS, Zero, FUN0)
                            CreateBitField (OPTS, 0x05, FUN5)
                            CreateBitField (OPTS, 0x06, FUN6)
                            CreateBitField (OPTS, 0x08, FUN8)
                            CreateBitField (OPTS, 0x09, FUN9)
                            FUN0 = One
                            FUN5 = IORB /* \IORB */
                            If ((LTEN == One))
                            {
                                FUN6 = One
                            }

                            If ((Arg1 >= 0x02))
                            {
                                If (CondRefOf (ECR1))
                                {
                                    If ((ECR1 == One))
                                    {
                                        If ((Arg1 >= 0x03))
                                        {
                                            FUN8 = One
                                            FUN9 = One
                                        }
                                    }
                                }
                            }

                            ADBG (Concatenate ("TBT RP OPTS -", ToHexString (OPTS)))
                            Return (OPTS) /* \_SB_.PC00.TRP2._DSM.OPTS */
                        }
                        Case (0x05)
                        {
                            If ((Arg1 >= One))
                            {
                                Return (One)
                            }
                        }
                        Case (0x06)
                        {
                            If ((Arg1 >= 0x02))
                            {
                                LTRV [Zero] = ((LMSL >> 0x0A) & 0x07)
                                LTRV [One] = (LMSL & 0x03FF)
                                LTRV [0x02] = ((LNSL >> 0x0A) & 0x07)
                                LTRV [0x03] = (LNSL & 0x03FF)
                                Return (LTRV) /* \_SB_.PC00.TRP2.LTRV */
                            }
                        }
                        Case (0x08)
                        {
                            If (CondRefOf (ECR1))
                            {
                                If ((ECR1 == One))
                                {
                                    If ((Arg1 >= 0x03))
                                    {
                                        Return (One)
                                    }
                                }
                            }
                        }
                        Case (0x09)
                        {
                            If (CondRefOf (ECR1))
                            {
                                If ((ECR1 == One))
                                {
                                    If ((Arg1 >= 0x03))
                                    {
                                        Return (Package (0x05)
                                        {
                                            0xC350, 
                                            Ones, 
                                            Ones, 
                                            0xC350, 
                                            Ones
                                        })
                                    }
                                }
                            }
                        }

                    }
                }

                Return (Buffer (One)
                {
                     0x00                                             // .
                })
            }

            Device (PXSX)
            {
                Name (_ADR, Zero)  // _ADR: Address
                Method (_PRW, 0, NotSerialized)  // _PRW: Power Resources for Wake
                {
                    If (CondRefOf (GP1E))
                    {
                        If ((GP1E != Zero))
                        {
                            Return (GPRW (0xC7, 0x04))
                        }
                    }

                    Return (GPRW (0x69, 0x04))
                }
            }

            Method (_DSW, 3, NotSerialized)  // _DSW: Device Sleep Wake
            {
                ADBG (Concatenate ("TCSS RP _DSW TUID -", ToHexString (TUID)))
                ADBG (Concatenate ("Arg0 -", ToHexString (Arg0)))
                ADBG (Concatenate ("Arg1 -", ToHexString (Arg1)))
                \_SB.PC00.TDM0.SD3C = Arg1
                \_SB.PC00.TDM1.SD3C = Arg1
                ADBG (Concatenate ("TDM0 SD3C -", ToHexString (\_SB.PC00.TDM0.SD3C)))
                ADBG (Concatenate ("TDM1 SD3C -", ToHexString (\_SB.PC00.TDM1.SD3C)))
                C2PM (Arg0, Arg1, Arg2, DCPM)
            }

            Method (_PRW, 0, NotSerialized)  // _PRW: Power Resources for Wake
            {
                If (CondRefOf (GP1E))
                {
                    If ((GP1E != Zero))
                    {
                        Return (GPRW (0xC7, 0x04))
                    }
                }

                Return (GPRW (0x69, 0x04))
            }

            Method (LTRC, 0, Serialized)
            {
                ADBG (Concatenate ("TCSS RP LTREN check TUID -", ToHexString (TUID)))
                If ((VDID != 0xFFFFFFFF))
                {
                    ADBG (Concatenate ("Current DCTL2 LTREN -", ToHexString (LNRE)))
                    HPSX = One
                    If ((LASX == Zero))
                    {
                        LNRE = One
                        Sleep (One)
                        LNRE = Zero
                        ADBG (Concatenate ("Toggle LTREN to TUID -", ToHexString (TUID)))
                    }
                }
            }

            Method (CRPM, 0, Serialized)
            {
                If ((VDID != 0xFFFFFFFF))
                {
                    Return (D3HT) /* \_SB_.PC00.TRP2.D3HT */
                }

                Return (0xFF)
            }

            Method (HPEV, 0, Serialized)
            {
                If (((VDID != 0xFFFFFFFF) && HPSX))
                {
                    ADBG (Concatenate ("HotPlug Event Start for ITBT Port - ", ToHexString (TUID)))
                    If (((PDCX == One) && (DLSC == One)))
                    {
                        PDCX = One
                        HPSX = One
                        Notify (^, Zero) // Bus Check
                    }
                    Else
                    {
                        HPSX = One
                    }

                    ADBG (Concatenate ("HotPlug Event End for ITBT Port - ", ToHexString (TUID)))
                }
            }

            Name (STAT, One)
            Method (D3CX, 0, Serialized)
            {
                ADBG (Concatenate ("TBT RP D3ColdExit Start TUID -", ToHexString (TUID)))
                ADBG (Concatenate ("RP STAT -", ToHexString (STAT)))
                ADBG (Concatenate ("TBT RP VDID -", ToHexString (VDID)))
                ADBG (Concatenate ("VDID -", ToHexString (VDID)))
                If ((STAT == One))
                {
                    Return (Zero)
                }

                ADBG (Concatenate ("Original RPFE -", ToHexString (RPFE)))
                RPFE = Zero
                ADBG (Concatenate ("RPFE -", ToHexString (RPFE)))
                ADBG (Concatenate ("Original RPER -", ToHexString (RPER)))
                RPER = Zero
                ADBG (Concatenate ("RPER -", ToHexString (RPER)))
                L23R = One
                Local0 = Zero
                Local1 = L23R /* \_SB_.PC00.TRP2.L23R */
                While (Local1)
                {
                    If ((Local0 > 0x64))
                    {
                        Break
                    }

                    Local2 = Zero
                    While ((Local2 < 0x0A))
                    {
                        Stall (0x64)
                        Local2++
                    }

                    Local0++
                    Local1 = L23R /* \_SB_.PC00.TRP2.L23R */
                }

                ADBG (Concatenate ("L23R -", ToHexString (L23R)))
                ADBG (Concatenate ("Loop -", ToHexString (Local0)))
                STAT = One
                Local0 = Zero
                Local1 = LASX /* \_SB_.PC00.TRP2.LASX */
                If ((PLAT > Zero))
                {
                    While ((Local1 == Zero))
                    {
                        If ((Local0 > PLAT))
                        {
                            Break
                        }

                        Sleep (0x05)
                        Local0 += 0x05
                        Local1 = LASX /* \_SB_.PC00.TRP2.LASX */
                    }
                }

                ADBG (Concatenate ("LASX -", ToHexString (LASX)))
                ADBG (Concatenate ("PLAT -", ToHexString (PLAT)))
                ADBG ("TBT RP D3ColdExit End")
            }

            Method (D3CE, 0, Serialized)
            {
                ADBG (Concatenate ("TBT RP D3ColdEntry Start TUID -", ToHexString (TUID)))
                ADBG (Concatenate ("RP STAT -", ToHexString (STAT)))
                ADBG (Concatenate ("TBT RP VDID -", ToHexString (VDID)))
                If ((STAT == Zero))
                {
                    Return (Zero)
                }

                If (((LNSE == One) || (LSOE == One)))
                {
                    If (((LASX == One) && (LNRE == Zero)))
                    {
                        LNRE = One
                        Sleep (One)
                        LNRE = Zero
                        ADBG ("Toggle LNRE -")
                    }
                }

                L23E = One
                Local0 = Zero
                Local1 = L23E /* \_SB_.PC00.TRP2.L23E */
                While (Local1)
                {
                    If ((Local0 > 0x64))
                    {
                        Break
                    }

                    Local2 = Zero
                    While ((Local2 < 0x0A))
                    {
                        Stall (0x64)
                        Local2++
                    }

                    Local0++
                    Local1 = L23E /* \_SB_.PC00.TRP2.L23E */
                }

                ADBG (Concatenate ("L23E -", ToHexString (L23E)))
                ADBG (Concatenate ("Loop -", ToHexString (Local0)))
                STAT = Zero
                ADBG (Concatenate ("Original RPFE -", ToHexString (RPFE)))
                RPFE = One
                ADBG (Concatenate ("RPFE -", ToHexString (RPFE)))
                ADBG (Concatenate ("Original RPER -", ToHexString (RPER)))
                RPER = One
                ADBG (Concatenate ("RPER -", ToHexString (RPER)))
                ADBG (Concatenate ("TBT RP D3ColdEntry End TUID -", ToHexString (TUID)))
            }

            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
            {
                ADBG (Concatenate ("TBT _PS0 Start RP ", ToHexString (TUID)))
                ADBG (Concatenate ("TBT RP VDID -", ToHexString (VDID)))
                HPEV ()
                HPME ()
                If ((PMEX == One))
                {
                    ADBG ("Disable PME SCI")
                    PMEX = Zero
                }

                Sleep (PLAT)
                If (((TUID == Zero) || (TUID == One)))
                {
                    If ((\_SB.PC00.TDM0.WACT == One))
                    {
                        \_SB.PC00.TDM0.WACT = 0x02
                        \_SB.PC00.TDM0.WFCC (ITCT)
                        \_SB.PC00.TDM0.WACT = Zero
                    }
                    ElseIf ((\_SB.PC00.TDM0.WACT == 0x02))
                    {
                        ADBG ("Wait until other _PS0 get response")
                        While ((\_SB.PC00.TDM0.WACT != Zero))
                        {
                            Sleep (0x05)
                        }

                        ADBG ("Other _PS0 got response")
                    }
                }
                ElseIf ((\_SB.PC00.TDM1.WACT == One))
                {
                    \_SB.PC00.TDM1.WACT = 0x02
                    \_SB.PC00.TDM1.WFCC (ITCT)
                    \_SB.PC00.TDM1.WACT = Zero
                }
                ElseIf ((\_SB.PC00.TDM1.WACT == 0x02))
                {
                    ADBG ("Wait until other _PS0 get response")
                    While ((\_SB.PC00.TDM1.WACT != Zero))
                    {
                        Sleep (0x05)
                    }

                    ADBG ("Other _PS0 got response")
                }

                ADBG (Concatenate ("TBT _PS0 End RP ", ToHexString (TUID)))
            }

            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
            {
                ADBG (Concatenate ("TBT _PS3 Start RP ", ToHexString (TUID)))
                ADBG (Concatenate ("TBT RP VDID -", ToHexString (VDID)))
                CHKH ()
                If ((HPEX == Zero))
                {
                    ADBG ("Enable Hot Plug SCI")
                    HPEX = One
                    HPEV ()
                }

                If ((PMEX == Zero))
                {
                    ADBG ("Enable PME SCI")
                    PMEX = One
                    HPME ()
                }

                ADBG (Concatenate ("TBT _PS3 End RP ", ToHexString (TUID)))
            }

            Method (CHKH, 0, NotSerialized)
            {
                If ((PDCX == One))
                {
                    ADBG ("PDC get set before enable HotPlug SCI")
                    If ((DLSC == Zero))
                    {
                        ADBG ("Clear PDC since it is not a real hotplug")
                        PDCX = One
                    }
                }
            }

            Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
            {
                ADBG ("TBT RP _DSD")
                ADBG (Concatenate ("TUID = ", ToHexString (TUID)))
                If (CondRefOf (U4CM))
                {
                    If (((U4CM & 0x07) == Zero))
                    {
                        Return (Package (0x04)
                        {
                            ToUUID ("6211e2c0-58a3-4af3-90e1-927a4e0c55a4") /* Unknown UUID */, 
                            Package (0x01)
                            {
                                Package (0x02)
                                {
                                    "HotPlugSupportInD3", 
                                    One
                                }
                            }, 

                            ToUUID ("efcc06cc-73ac-4bc3-bff0-76143807c389") /* Unknown UUID */, 
                            Package (0x02)
                            {
                                Package (0x02)
                                {
                                    "ExternalFacingPort", 
                                    One
                                }, 

                                Package (0x02)
                                {
                                    "UID", 
                                    TUID
                                }
                            }
                        })
                    }
                }

                If ((((TUID == Zero) || (TUID == One)) && (DME0 == One)))
                {
                    Return (Package (0x06)
                    {
                        ToUUID ("6211e2c0-58a3-4af3-90e1-927a4e0c55a4") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "HotPlugSupportInD3", 
                                One
                            }
                        }, 

                        ToUUID ("efcc06cc-73ac-4bc3-bff0-76143807c389") /* Unknown UUID */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "ExternalFacingPort", 
                                One
                            }, 

                            Package (0x02)
                            {
                                "UID", 
                                TUID
                            }
                        }, 

                        ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "usb4-host-interface", 
                                \_SB.PC00.TDM0
                            }, 

                            Package (0x02)
                            {
                                "usb4-port-number", 
                                TUID
                            }
                        }
                    })
                }
                ElseIf ((((TUID == 0x02) || (TUID == 0x03)) && (DME1 == 
                    One)))
                {
                    Return (Package (0x06)
                    {
                        ToUUID ("6211e2c0-58a3-4af3-90e1-927a4e0c55a4") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "HotPlugSupportInD3", 
                                One
                            }
                        }, 

                        ToUUID ("efcc06cc-73ac-4bc3-bff0-76143807c389") /* Unknown UUID */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "ExternalFacingPort", 
                                One
                            }, 

                            Package (0x02)
                            {
                                "UID", 
                                TUID
                            }
                        }, 

                        ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "usb4-host-interface", 
                                \_SB.PC00.TDM1
                            }, 

                            Package (0x02)
                            {
                                "usb4-port-number", 
                                TUID
                            }
                        }
                    })
                }
                Else
                {
                    Return (Package (0x04)
                    {
                        ToUUID ("6211e2c0-58a3-4af3-90e1-927a4e0c55a4") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "HotPlugSupportInD3", 
                                One
                            }
                        }, 

                        ToUUID ("efcc06cc-73ac-4bc3-bff0-76143807c389") /* Unknown UUID */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "ExternalFacingPort", 
                                One
                            }, 

                            Package (0x02)
                            {
                                "UID", 
                                TUID
                            }
                        }
                    })
                }
            }

            Method (_S0W, 0, NotSerialized)  // _S0W: S0 Device Wake State
            {
                If (ITRT)
                {
                    Return (0x04)
                }
                Else
                {
                    Return (0x03)
                }
            }

            If (ITRT)
            {
                Method (_PR0, 0, NotSerialized)  // _PR0: Power Resources for D0
                {
                    If (TRTD)
                    {
                        If (((\_SB.PC00.TDM0.SCME == One) || (\_SB.PC00.TDM1.SCME == One)))
                        {
                            Return (Package (0x01)
                            {
                                \_SB.PC00.D3C
                            })
                        }

                        If (((TUID == Zero) || (TUID == One)))
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT0
                            })
                        }
                        Else
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT1
                            })
                        }
                    }

                    If (((TUID == Zero) || (TUID == One)))
                    {
                        Return (Package (0x01)
                        {
                            \_SB.PC00.TBT0
                        })
                    }

                    Return (Package (0x01)
                    {
                        \_SB.PC00.TBT1
                    })
                }

                Method (_PR3, 0, NotSerialized)  // _PR3: Power Resources for D3hot
                {
                    If (TRTD)
                    {
                        If (((\_SB.PC00.TDM0.SCME == One) || (\_SB.PC00.TDM1.SCME == One)))
                        {
                            Return (Package (0x01)
                            {
                                \_SB.PC00.D3C
                            })
                        }

                        If (((TUID == Zero) || (TUID == One)))
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT0
                            })
                        }
                        Else
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT1
                            })
                        }
                    }

                    If (((TUID == Zero) || (TUID == One)))
                    {
                        Return (Package (0x01)
                        {
                            \_SB.PC00.TBT0
                        })
                    }

                    Return (Package (0x01)
                    {
                        \_SB.PC00.TBT1
                    })
                }
            }

            Method (HPME, 0, Serialized)
            {
                ADBG ("TBT HPME")
                If (((VDID != 0xFFFFFFFF) && (PMSX == One)))
                {
                    ADBG ("TBT RP exists")
                    Notify (PXSX, 0x02) // Device Wake
                    PMSX = One
                    PSPX = One
                    Return (One)
                }

                Return (Zero)
            }

            Name (AR01, Package (0x04)
            {
                Package (0x04)
                {
                    0xFFFF, 
                    Zero, 
                    Zero, 
                    0x10
                }, 

                Package (0x04)
                {
                    0xFFFF, 
                    One, 
                    Zero, 
                    0x11
                }, 

                Package (0x04)
                {
                    0xFFFF, 
                    0x02, 
                    Zero, 
                    0x12
                }, 

                Package (0x04)
                {
                    0xFFFF, 
                    0x03, 
                    Zero, 
                    0x13
                }
            })
            Method (_PRT, 0, NotSerialized)  // _PRT: PCI Routing Table
            {
                Return (AR01) /* \_SB_.PC00.TRP2.AR01 */
            }
        }

        Device (TRP3)
        {
            Method (_ADR, 0, NotSerialized)  // _ADR: Address
            {
                If ((TPA3 != Zero))
                {
                    ADBG (Concatenate ("TRP3 _ADR", ToHexString (TPA3)))
                    Return (TPA3) /* External reference */
                }
                Else
                {
                    Return (0x00070003)
                }
            }

            Name (TUID, 0x03)
            Name (LTEN, Zero)
            Name (LMSL, 0x88C8)
            Name (LNSL, 0x88C8)
            Name (DCPM, 0x03)
            Method (_STA, 0, NotSerialized)  // _STA: Status
            {
                ADBG (Concatenate ("TCSS RP3 _STA", ToHexString (TRE3)))
                If ((TRE3 == One))
                {
                    Return (0x0F)
                }

                Return (Zero)
            }

            Method (_INI, 0, NotSerialized)  // _INI: Initialize
            {
                LTEN = LTE3 /* External reference */
                LMSL = PSL3 /* External reference */
                LNSL = PNS3 /* External reference */
            }

            Name (PRTP, 0x08)
            Name (BCFG, Ones)
            Method (BASE, 0, NotSerialized)
            {
                If ((BCFG != Ones))
                {
                    ADBG (Concatenate ("PCIe MMIO Address ", ToHexString (BCFG)))
                    Return (BCFG) /* \_SB_.PC00.TRP3.BCFG */
                }
                Else
                {
                    Local0 = (_ADR () & 0x07)
                    Local1 = ((_ADR () >> 0x10) & 0x1F)
                    ADBG (Concatenate ("BASE of ITBT Port ", ToHexString (TUID)))
                    Local2 = ((Local0 << 0x0C) + (Local1 << 0x0F))
                    Local3 = (\_SB.PC00.GPCB () + Local2)
                    BCFG = Local3
                    ADBG (Concatenate ("PCIe MMIO Address ", ToHexString (BCFG)))
                    Return (BCFG) /* \_SB_.PC00.TRP3.BCFG */
                }
            }

            Method (GOPB, 1, NotSerialized)
            {
                Local0 = (BASE () + Arg0)
                ADBG (Concatenate ("TCSS PCIE RP register offset MMIO Address ", ToHexString (Local0)))
                Return (Local0)
            }

            OperationRegion (PXCS, SystemMemory, BASE (), 0x0C00)
            Field (PXCS, AnyAcc, NoLock, Preserve)
            {
                VDID,   32, 
                Offset (0x50), 
                L0SE,   1, 
                    ,   3, 
                LDIS,   1, 
                Offset (0x51), 
                Offset (0x52), 
                    ,   13, 
                LASX,   1, 
                Offset (0x5A), 
                ABPX,   1, 
                    ,   2, 
                PDCX,   1, 
                    ,   2, 
                PDSX,   1, 
                Offset (0x5B), 
                DLSC,   1, 
                Offset (0x60), 
                Offset (0x62), 
                PSPX,   1, 
                Offset (0xA4), 
                D3HT,   2, 
                Offset (0x328), 
                STS1,   32
            }

            OperationRegion (TPCS, SystemMemory, GOPB (0x0420), 0x04)
            Field (TPCS, AnyAcc, NoLock, Preserve)
            {
                    ,   30, 
                DPGE,   1
            }

            OperationRegion (ADCS, SystemMemory, GOPB (0x05BC), 0x04)
            Field (ADCS, AnyAcc, NoLock, Preserve)
            {
                    ,   3, 
                RPER,   1, 
                RPFE,   1
            }

            OperationRegion (CCCS, SystemMemory, GOPB (RPCC), 0x20)
            Field (CCCS, AnyAcc, NoLock, WriteAsZeros)
            {
                Offset (0x0C), 
                    ,   30, 
                HPSX,   1, 
                PMSX,   1
            }

            Field (CCCS, AnyAcc, NoLock, Preserve)
            {
                Offset (0x08), 
                    ,   30, 
                HPEX,   1, 
                PMEX,   1, 
                Offset (0x12), 
                    ,   2, 
                L23E,   1, 
                L23R,   1
            }

            Field (PXCS, AnyAcc, NoLock, Preserve)
            {
                Offset (0x68), 
                    ,   10, 
                LNRE,   1, 
                Offset (0x404), 
                LSOE,   1, 
                LNSE,   1
            }

            Name (LTRV, Package (0x04)
            {
                Zero, 
                Zero, 
                Zero, 
                Zero
            })
            Method (_DSM, 4, Serialized)  // _DSM: Device-Specific Method
            {
                If ((Arg0 == ToUUID ("e5c937d0-3553-4d7a-9117-ea4d19c3434d") /* Device Labeling Interface */))
                {
                    Switch (ToInteger (Arg2))
                    {
                        Case (Zero)
                        {
                            Name (OPTS, Buffer (0x02)
                            {
                                 0x00, 0x00                                       // ..
                            })
                            CreateBitField (OPTS, Zero, FUN0)
                            CreateBitField (OPTS, 0x05, FUN5)
                            CreateBitField (OPTS, 0x06, FUN6)
                            CreateBitField (OPTS, 0x08, FUN8)
                            CreateBitField (OPTS, 0x09, FUN9)
                            FUN0 = One
                            FUN5 = IORB /* \IORB */
                            If ((LTEN == One))
                            {
                                FUN6 = One
                            }

                            If ((Arg1 >= 0x02))
                            {
                                If (CondRefOf (ECR1))
                                {
                                    If ((ECR1 == One))
                                    {
                                        If ((Arg1 >= 0x03))
                                        {
                                            FUN8 = One
                                            FUN9 = One
                                        }
                                    }
                                }
                            }

                            ADBG (Concatenate ("TBT RP OPTS -", ToHexString (OPTS)))
                            Return (OPTS) /* \_SB_.PC00.TRP3._DSM.OPTS */
                        }
                        Case (0x05)
                        {
                            If ((Arg1 >= One))
                            {
                                Return (One)
                            }
                        }
                        Case (0x06)
                        {
                            If ((Arg1 >= 0x02))
                            {
                                LTRV [Zero] = ((LMSL >> 0x0A) & 0x07)
                                LTRV [One] = (LMSL & 0x03FF)
                                LTRV [0x02] = ((LNSL >> 0x0A) & 0x07)
                                LTRV [0x03] = (LNSL & 0x03FF)
                                Return (LTRV) /* \_SB_.PC00.TRP3.LTRV */
                            }
                        }
                        Case (0x08)
                        {
                            If (CondRefOf (ECR1))
                            {
                                If ((ECR1 == One))
                                {
                                    If ((Arg1 >= 0x03))
                                    {
                                        Return (One)
                                    }
                                }
                            }
                        }
                        Case (0x09)
                        {
                            If (CondRefOf (ECR1))
                            {
                                If ((ECR1 == One))
                                {
                                    If ((Arg1 >= 0x03))
                                    {
                                        Return (Package (0x05)
                                        {
                                            0xC350, 
                                            Ones, 
                                            Ones, 
                                            0xC350, 
                                            Ones
                                        })
                                    }
                                }
                            }
                        }

                    }
                }

                Return (Buffer (One)
                {
                     0x00                                             // .
                })
            }

            Device (PXSX)
            {
                Name (_ADR, Zero)  // _ADR: Address
                Method (_PRW, 0, NotSerialized)  // _PRW: Power Resources for Wake
                {
                    If (CondRefOf (GP1E))
                    {
                        If ((GP1E != Zero))
                        {
                            Return (GPRW (0xC7, 0x04))
                        }
                    }

                    Return (GPRW (0x69, 0x04))
                }
            }

            Method (_DSW, 3, NotSerialized)  // _DSW: Device Sleep Wake
            {
                ADBG (Concatenate ("TCSS RP _DSW TUID -", ToHexString (TUID)))
                ADBG (Concatenate ("Arg0 -", ToHexString (Arg0)))
                ADBG (Concatenate ("Arg1 -", ToHexString (Arg1)))
                \_SB.PC00.TDM0.SD3C = Arg1
                \_SB.PC00.TDM1.SD3C = Arg1
                ADBG (Concatenate ("TDM0 SD3C -", ToHexString (\_SB.PC00.TDM0.SD3C)))
                ADBG (Concatenate ("TDM1 SD3C -", ToHexString (\_SB.PC00.TDM1.SD3C)))
                C2PM (Arg0, Arg1, Arg2, DCPM)
            }

            Method (_PRW, 0, NotSerialized)  // _PRW: Power Resources for Wake
            {
                If (CondRefOf (GP1E))
                {
                    If ((GP1E != Zero))
                    {
                        Return (GPRW (0xC7, 0x04))
                    }
                }

                Return (GPRW (0x69, 0x04))
            }

            Method (LTRC, 0, Serialized)
            {
                ADBG (Concatenate ("TCSS RP LTREN check TUID -", ToHexString (TUID)))
                If ((VDID != 0xFFFFFFFF))
                {
                    ADBG (Concatenate ("Current DCTL2 LTREN -", ToHexString (LNRE)))
                    HPSX = One
                    If ((LASX == Zero))
                    {
                        LNRE = One
                        Sleep (One)
                        LNRE = Zero
                        ADBG (Concatenate ("Toggle LTREN to TUID -", ToHexString (TUID)))
                    }
                }
            }

            Method (CRPM, 0, Serialized)
            {
                If ((VDID != 0xFFFFFFFF))
                {
                    Return (D3HT) /* \_SB_.PC00.TRP3.D3HT */
                }

                Return (0xFF)
            }

            Method (HPEV, 0, Serialized)
            {
                If (((VDID != 0xFFFFFFFF) && HPSX))
                {
                    ADBG (Concatenate ("HotPlug Event Start for ITBT Port - ", ToHexString (TUID)))
                    If (((PDCX == One) && (DLSC == One)))
                    {
                        PDCX = One
                        HPSX = One
                        Notify (^, Zero) // Bus Check
                    }
                    Else
                    {
                        HPSX = One
                    }

                    ADBG (Concatenate ("HotPlug Event End for ITBT Port - ", ToHexString (TUID)))
                }
            }

            Name (STAT, One)
            Method (D3CX, 0, Serialized)
            {
                ADBG (Concatenate ("TBT RP D3ColdExit Start TUID -", ToHexString (TUID)))
                ADBG (Concatenate ("RP STAT -", ToHexString (STAT)))
                ADBG (Concatenate ("TBT RP VDID -", ToHexString (VDID)))
                ADBG (Concatenate ("VDID -", ToHexString (VDID)))
                If ((STAT == One))
                {
                    Return (Zero)
                }

                ADBG (Concatenate ("Original RPFE -", ToHexString (RPFE)))
                RPFE = Zero
                ADBG (Concatenate ("RPFE -", ToHexString (RPFE)))
                ADBG (Concatenate ("Original RPER -", ToHexString (RPER)))
                RPER = Zero
                ADBG (Concatenate ("RPER -", ToHexString (RPER)))
                L23R = One
                Local0 = Zero
                Local1 = L23R /* \_SB_.PC00.TRP3.L23R */
                While (Local1)
                {
                    If ((Local0 > 0x64))
                    {
                        Break
                    }

                    Local2 = Zero
                    While ((Local2 < 0x0A))
                    {
                        Stall (0x64)
                        Local2++
                    }

                    Local0++
                    Local1 = L23R /* \_SB_.PC00.TRP3.L23R */
                }

                ADBG (Concatenate ("L23R -", ToHexString (L23R)))
                ADBG (Concatenate ("Loop -", ToHexString (Local0)))
                STAT = One
                Local0 = Zero
                Local1 = LASX /* \_SB_.PC00.TRP3.LASX */
                If ((PLAT > Zero))
                {
                    While ((Local1 == Zero))
                    {
                        If ((Local0 > PLAT))
                        {
                            Break
                        }

                        Sleep (0x05)
                        Local0 += 0x05
                        Local1 = LASX /* \_SB_.PC00.TRP3.LASX */
                    }
                }

                ADBG (Concatenate ("LASX -", ToHexString (LASX)))
                ADBG (Concatenate ("PLAT -", ToHexString (PLAT)))
                ADBG ("TBT RP D3ColdExit End")
            }

            Method (D3CE, 0, Serialized)
            {
                ADBG (Concatenate ("TBT RP D3ColdEntry Start TUID -", ToHexString (TUID)))
                ADBG (Concatenate ("RP STAT -", ToHexString (STAT)))
                ADBG (Concatenate ("TBT RP VDID -", ToHexString (VDID)))
                If ((STAT == Zero))
                {
                    Return (Zero)
                }

                If (((LNSE == One) || (LSOE == One)))
                {
                    If (((LASX == One) && (LNRE == Zero)))
                    {
                        LNRE = One
                        Sleep (One)
                        LNRE = Zero
                        ADBG ("Toggle LNRE -")
                    }
                }

                L23E = One
                Local0 = Zero
                Local1 = L23E /* \_SB_.PC00.TRP3.L23E */
                While (Local1)
                {
                    If ((Local0 > 0x64))
                    {
                        Break
                    }

                    Local2 = Zero
                    While ((Local2 < 0x0A))
                    {
                        Stall (0x64)
                        Local2++
                    }

                    Local0++
                    Local1 = L23E /* \_SB_.PC00.TRP3.L23E */
                }

                ADBG (Concatenate ("L23E -", ToHexString (L23E)))
                ADBG (Concatenate ("Loop -", ToHexString (Local0)))
                STAT = Zero
                ADBG (Concatenate ("Original RPFE -", ToHexString (RPFE)))
                RPFE = One
                ADBG (Concatenate ("RPFE -", ToHexString (RPFE)))
                ADBG (Concatenate ("Original RPER -", ToHexString (RPER)))
                RPER = One
                ADBG (Concatenate ("RPER -", ToHexString (RPER)))
                ADBG (Concatenate ("TBT RP D3ColdEntry End TUID -", ToHexString (TUID)))
            }

            Method (_PS0, 0, Serialized)  // _PS0: Power State 0
            {
                ADBG (Concatenate ("TBT _PS0 Start RP ", ToHexString (TUID)))
                ADBG (Concatenate ("TBT RP VDID -", ToHexString (VDID)))
                HPEV ()
                HPME ()
                If ((PMEX == One))
                {
                    ADBG ("Disable PME SCI")
                    PMEX = Zero
                }

                Sleep (PLAT)
                If (((TUID == Zero) || (TUID == One)))
                {
                    If ((\_SB.PC00.TDM0.WACT == One))
                    {
                        \_SB.PC00.TDM0.WACT = 0x02
                        \_SB.PC00.TDM0.WFCC (ITCT)
                        \_SB.PC00.TDM0.WACT = Zero
                    }
                    ElseIf ((\_SB.PC00.TDM0.WACT == 0x02))
                    {
                        ADBG ("Wait until other _PS0 get response")
                        While ((\_SB.PC00.TDM0.WACT != Zero))
                        {
                            Sleep (0x05)
                        }

                        ADBG ("Other _PS0 got response")
                    }
                }
                ElseIf ((\_SB.PC00.TDM1.WACT == One))
                {
                    \_SB.PC00.TDM1.WACT = 0x02
                    \_SB.PC00.TDM1.WFCC (ITCT)
                    \_SB.PC00.TDM1.WACT = Zero
                }
                ElseIf ((\_SB.PC00.TDM1.WACT == 0x02))
                {
                    ADBG ("Wait until other _PS0 get response")
                    While ((\_SB.PC00.TDM1.WACT != Zero))
                    {
                        Sleep (0x05)
                    }

                    ADBG ("Other _PS0 got response")
                }

                ADBG (Concatenate ("TBT _PS0 End RP ", ToHexString (TUID)))
            }

            Method (_PS3, 0, Serialized)  // _PS3: Power State 3
            {
                ADBG (Concatenate ("TBT _PS3 Start RP ", ToHexString (TUID)))
                ADBG (Concatenate ("TBT RP VDID -", ToHexString (VDID)))
                CHKH ()
                If ((HPEX == Zero))
                {
                    ADBG ("Enable Hot Plug SCI")
                    HPEX = One
                    HPEV ()
                }

                If ((PMEX == Zero))
                {
                    ADBG ("Enable PME SCI")
                    PMEX = One
                    HPME ()
                }

                ADBG (Concatenate ("TBT _PS3 End RP ", ToHexString (TUID)))
            }

            Method (CHKH, 0, NotSerialized)
            {
                If ((PDCX == One))
                {
                    ADBG ("PDC get set before enable HotPlug SCI")
                    If ((DLSC == Zero))
                    {
                        ADBG ("Clear PDC since it is not a real hotplug")
                        PDCX = One
                    }
                }
            }

            Method (_DSD, 0, NotSerialized)  // _DSD: Device-Specific Data
            {
                ADBG ("TBT RP _DSD")
                ADBG (Concatenate ("TUID = ", ToHexString (TUID)))
                If (CondRefOf (U4CM))
                {
                    If (((U4CM & 0x07) == Zero))
                    {
                        Return (Package (0x04)
                        {
                            ToUUID ("6211e2c0-58a3-4af3-90e1-927a4e0c55a4") /* Unknown UUID */, 
                            Package (0x01)
                            {
                                Package (0x02)
                                {
                                    "HotPlugSupportInD3", 
                                    One
                                }
                            }, 

                            ToUUID ("efcc06cc-73ac-4bc3-bff0-76143807c389") /* Unknown UUID */, 
                            Package (0x02)
                            {
                                Package (0x02)
                                {
                                    "ExternalFacingPort", 
                                    One
                                }, 

                                Package (0x02)
                                {
                                    "UID", 
                                    TUID
                                }
                            }
                        })
                    }
                }

                If ((((TUID == Zero) || (TUID == One)) && (DME0 == One)))
                {
                    Return (Package (0x06)
                    {
                        ToUUID ("6211e2c0-58a3-4af3-90e1-927a4e0c55a4") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "HotPlugSupportInD3", 
                                One
                            }
                        }, 

                        ToUUID ("efcc06cc-73ac-4bc3-bff0-76143807c389") /* Unknown UUID */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "ExternalFacingPort", 
                                One
                            }, 

                            Package (0x02)
                            {
                                "UID", 
                                TUID
                            }
                        }, 

                        ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "usb4-host-interface", 
                                \_SB.PC00.TDM0
                            }, 

                            Package (0x02)
                            {
                                "usb4-port-number", 
                                TUID
                            }
                        }
                    })
                }
                ElseIf ((((TUID == 0x02) || (TUID == 0x03)) && (DME1 == 
                    One)))
                {
                    Return (Package (0x06)
                    {
                        ToUUID ("6211e2c0-58a3-4af3-90e1-927a4e0c55a4") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "HotPlugSupportInD3", 
                                One
                            }
                        }, 

                        ToUUID ("efcc06cc-73ac-4bc3-bff0-76143807c389") /* Unknown UUID */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "ExternalFacingPort", 
                                One
                            }, 

                            Package (0x02)
                            {
                                "UID", 
                                TUID
                            }
                        }, 

                        ToUUID ("daffd814-6eba-4d8c-8a91-bc9bbf4aa301") /* Device Properties for _DSD */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "usb4-host-interface", 
                                \_SB.PC00.TDM1
                            }, 

                            Package (0x02)
                            {
                                "usb4-port-number", 
                                TUID
                            }
                        }
                    })
                }
                Else
                {
                    Return (Package (0x04)
                    {
                        ToUUID ("6211e2c0-58a3-4af3-90e1-927a4e0c55a4") /* Unknown UUID */, 
                        Package (0x01)
                        {
                            Package (0x02)
                            {
                                "HotPlugSupportInD3", 
                                One
                            }
                        }, 

                        ToUUID ("efcc06cc-73ac-4bc3-bff0-76143807c389") /* Unknown UUID */, 
                        Package (0x02)
                        {
                            Package (0x02)
                            {
                                "ExternalFacingPort", 
                                One
                            }, 

                            Package (0x02)
                            {
                                "UID", 
                                TUID
                            }
                        }
                    })
                }
            }

            Method (_S0W, 0, NotSerialized)  // _S0W: S0 Device Wake State
            {
                If (ITRT)
                {
                    Return (0x04)
                }
                Else
                {
                    Return (0x03)
                }
            }

            If (ITRT)
            {
                Method (_PR0, 0, NotSerialized)  // _PR0: Power Resources for D0
                {
                    If (TRTD)
                    {
                        If (((\_SB.PC00.TDM0.SCME == One) || (\_SB.PC00.TDM1.SCME == One)))
                        {
                            Return (Package (0x01)
                            {
                                \_SB.PC00.D3C
                            })
                        }

                        If (((TUID == Zero) || (TUID == One)))
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT0
                            })
                        }
                        Else
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT1
                            })
                        }
                    }

                    If (((TUID == Zero) || (TUID == One)))
                    {
                        Return (Package (0x01)
                        {
                            \_SB.PC00.TBT0
                        })
                    }

                    Return (Package (0x01)
                    {
                        \_SB.PC00.TBT1
                    })
                }

                Method (_PR3, 0, NotSerialized)  // _PR3: Power Resources for D3hot
                {
                    If (TRTD)
                    {
                        If (((\_SB.PC00.TDM0.SCME == One) || (\_SB.PC00.TDM1.SCME == One)))
                        {
                            Return (Package (0x01)
                            {
                                \_SB.PC00.D3C
                            })
                        }

                        If (((TUID == Zero) || (TUID == One)))
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT0
                            })
                        }
                        Else
                        {
                            Return (Package (0x02)
                            {
                                \_SB.PC00.D3C, 
                                \_SB.PC00.TBT1
                            })
                        }
                    }

                    If (((TUID == Zero) || (TUID == One)))
                    {
                        Return (Package (0x01)
                        {
                            \_SB.PC00.TBT0
                        })
                    }

                    Return (Package (0x01)
                    {
                        \_SB.PC00.TBT1
                    })
                }
            }

            Method (HPME, 0, Serialized)
            {
                ADBG ("TBT HPME")
                If (((VDID != 0xFFFFFFFF) && (PMSX == One)))
                {
                    ADBG ("TBT RP exists")
                    Notify (PXSX, 0x02) // Device Wake
                    PMSX = One
                    PSPX = One
                    Return (One)
                }

                Return (Zero)
            }

            Name (AR01, Package (0x04)
            {
                Package (0x04)
                {
                    0xFFFF, 
                    Zero, 
                    Zero, 
                    0x10
                }, 

                Package (0x04)
                {
                    0xFFFF, 
                    One, 
                    Zero, 
                    0x11
                }, 

                Package (0x04)
                {
                    0xFFFF, 
                    0x02, 
                    Zero, 
                    0x12
                }, 

                Package (0x04)
                {
                    0xFFFF, 
                    0x03, 
                    Zero, 
                    0x13
                }
            })
            Method (_PRT, 0, NotSerialized)  // _PRT: PCI Routing Table
            {
                Return (AR01) /* \_SB_.PC00.TRP3.AR01 */
            }
        }
    }
}

