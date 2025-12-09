import http.server
import socketserver
import json
import os
import sys

DEFAULT_PORT = 6657

class Handler(http.server.SimpleHTTPRequestHandler):
    def do_POST(self):
        if self.path == '/encode':
            self.handle_encode()
        elif self.path == '/decode':
            self.handle_decode()
        elif self.path == '/health':
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"OK")
        else:
            self.send_error(404)

    def handle_encode(self):
        # Expect JSON meta in header or initial body?
        # Simplified: Rust sends JSON body with paths.
        content_len = int(self.headers.get('Content-Length'))
        post_body = self.rfile.read(content_len)
        data = json.loads(post_body)
        
        container = data['container']
        output = data['output']
        # Payload stream is not handled here in this simple MVP path-based version.
        # BUT Sound_PNG architecture passes a Stream to the plugin.
        # The plugin (Rust) needs to read the stream and either:
        # 1. Save to temp file, pass path to Python.
        # 2. Stream to Python.
        
        # If we assume Rust plugin saved the stream to a temp file (which EncodeStream does NOT do, it gives a ByteStream),
        # The Rust plugin must read ByteStream and write to a temp file if Python expects a file.
        # OR Python accepts the stream in the POST body.
        
        # Let's assume Rust plugin writes the payload to a temp file and passes it here.
        payload_tmp = data['payload_tmp']
        
        # Mock Processing (Copy container to output)
        # Real implementation would use cv2/numpy etc.
        # Here we just "touch" the output or copy container
        try:
            with open(container, 'rb') as f:
                content = f.read()
            with open(output, 'wb') as f:
                f.write(content)
                # Append payload for "steganography"
                with open(payload_tmp, 'rb') as p:
                    f.write(p.read()) # Naive append
            
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"Encoded")
        except Exception as e:
            self.send_error(500, str(e))

    def handle_decode(self):
        content_len = int(self.headers.get('Content-Length'))
        post_body = self.rfile.read(content_len)
        data = json.loads(post_body)
        
        input_path = data['input']
        # Output is a stream in Rust trait.
        # Python must return the data stream.
        
        try:
            # Mock: Read end of file
            with open(input_path, 'rb') as f:
                f.seek(0, 2)
                size = f.tell()
                # Assume last 1KB is payload if we just appended
                # This is a mock.
                f.seek(max(0, size - 1024))
                data = f.read()
            
            self.send_response(200)
            self.end_headers()
            self.wfile.write(data)
        except Exception as e:
            self.send_error(500, str(e))

if __name__ == "__main__":
    port = DEFAULT_PORT
    if len(sys.argv) > 1:
        try:
            port = int(sys.argv[1])
        except ValueError:
            print(f"Invalid port argument, using default {DEFAULT_PORT}")
            
    with socketserver.TCPServer(("", port), Handler) as httpd:
        print(f"Serving at port {port}")
        httpd.serve_forever()
