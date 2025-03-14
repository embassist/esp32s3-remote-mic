import socket

UDP_IP = "192.168.244.55"  # Replace with ESP32's actual IP
UDP_PORT = 8080

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.sendto(b"Ready", (UDP_IP, UDP_PORT))

while True:
    data, addr = sock.recvfrom(1024)
    print(f"Received: {data.decode()}")