//go:build pca10040

package main

import "machine"

const (
	button          = machine.BUTTON
	buttonMode      = machine.PinInputPullup
	buttonPinChange = machine.PinRising
)
