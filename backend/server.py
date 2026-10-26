import os
import sys
import signal
import threading
from flask import Flask, request, jsonify
from waitress import serve
from bertopic import BERTopic

app = Flask(__name__)

def monitor_parent():
    sys.stdin.buffer.read()
    print("Node disconnected. Exiting", flush=True)
    os.kill(os.getpid(), signal.SIGTERM)

threading.Thread(
    target=monitor_parent,
    daemon=True
).start()

@app.get("/health")
def health():
    return jsonify({"status": "ready"})

@app.post("/topics")
def extract_topics():
    data = request.get_json()
    documents = data["documents"]

    model = BERTopic()
    topics, _ = model.fit_transform(documents)

    return jsonify({
        "topics": [int(topic) for topic in topics]
    })

if __name__ == "__main__":
    serve(app, host="127.0.0.1", port=0, use_reloader=False)
