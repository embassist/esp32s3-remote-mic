import socket

UDP_IP = "192.168.177.55"
UDP_PORT = 8080

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.connect((UDP_IP, UDP_PORT))
# sock.sendto(b"\x00", (UDP_IP, UDP_PORT))
print('ok')
while True:
    data, addr = sock.recvfrom(1024)
    print(f"Received: {data.decode()}")
