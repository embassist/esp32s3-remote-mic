ffmpeg -f s16le -ar 8000 -ac 1 -i udp://192.168.244.55:8080 -f wav test.wav