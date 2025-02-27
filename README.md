# Setup:

- hardware
    - analogue microphone, can be:
        - MAX4466 with any of:
            - PCM1808 (Good general-purpose I2S ADC)
            - ES7210 (Multiple mic inputs, good for stereo)
            - MAX9814 (Has a built-in mic preamp and AGC)
        - BoyaM1 with ADC
        - 2 pin from headphones, but external preamp
    - solder mic with esp32c3 or esp32s3
    - power up the esp with 26650 using tp4056 for ~15-20 hours uptime
- software
    - flash the esp with rust
    - read from mic using esp32s3 via I2S
    - send on:
        - udp for lowest latency
        - ble for low-energy
