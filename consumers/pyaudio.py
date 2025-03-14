import struct
import pyaudio

stream = pyaudio.PyAudio().open(
    format=pyaudio.paInt16,
    channels=1,
    rate=8000,
    output=True
)

while True:
    data, _ = sock.recvfrom(4096)
    stream.write(data)