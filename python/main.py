import socket
from threading import Thread

HOST = "127.0.0.1"
PORT = 6379
COMMANDS = ["echo", "get", "set", "ping"]


def encode_to_resp(string):
    parts = string.split(" ")
    if len(parts) <= 2:
        result = ""
    elif len(parts) > 2:
        result = "*" + str(len(parts)) + "\r\n"

    for part in parts:
        result = result + "$" + str(len(part)) + "\r\n" + part + "\r\n"

    return result.encode()


def decode_from_resp_array(arr):
    if arr[0][0] == "*":
        length = int(arr[0][1])

    parsed = []
    idx = 1
    for _ in range(length):
        current = arr[idx]
        if current[0] == "$":
            idx += 1
            current = arr[idx]
            parsed.append(current.lower())
        idx += 1
    return parsed


def parse_resp(string: bytes):
    parts = string.decode().split("\r\n")[:-1]
    parsed = decode_from_resp_array(parts)
    print(parsed)
    return parsed


def reply(parsed, client_socket):
    if parsed[0] == "echo":
        client_socket.sendall(encode_to_resp(parsed[1]))
    elif parsed[0] == "command":
        client_socket.sendall(encode_to_resp(""))
    else:
        return


def handle_client(client_socket):
    try:
        while True:
            data = client_socket.recv(1024)
            if not data:
                break
            parsed = parse_resp(data)
            reply(parsed, client_socket)
    except ConnectionError:
        print("Client Disconnected")
    finally:
        client_socket.close()


def server():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as server_socket:
        server_socket.bind((HOST, PORT))
        server_socket.listen()

        print(f"Server listening on {HOST}:{PORT}")

        while True:
            client_socket, client_address = server_socket.accept()
            print(f"Connection from {client_address}")
            thread = Thread(target=handle_client, args=[client_socket])
            thread.start()


if __name__ == "__main__":
    server()
