# ESP32-C3 RTIC template

This crate showcases a simple RTIC application for RT-Ibex.

## Prerequisites

### Our custom toolchain

Sa. [Requirements for compiling Rust](../README.md#requirements-for-compiling-rust)

## Running the crate

`just run`, see Justfile for details.

### Expected behavior

The program

- Prints ``init``
- Enters a high prio task
- During the execution of the high prio task, the button should be non-functional
- Pends a low prio task
- Exits the high prio task
- Enters the low prio task
- During the execution of the low prio task, the button should be functional.
- Exits the low prio task
- Prints ``idle``
