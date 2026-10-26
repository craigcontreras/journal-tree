import os
import sys
import signal
import logging
import threading
from flask import Flask, request, jsonify
from waitress import create_server
from bertopic import BERTopic
import pandas as pd

pd.set_option("display.max_rows", None)
pd.set_option("display.max_columns", None)

logging.basicConfig(
    level = logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s"
)
logger = logging.getLogger(__name__)
logging.getLogger("BERTopic").setLevel(logging.INFO)
app = Flask(__name__)

@app.get("/health")
def health():
    return jsonify({"status": "ready"})

@app.post("/topics")
def extract_topics():
    data = request.get_json()
    documents = data["documents"]

    model = BERTopic(verbose=True)
    topics, _ = model.fit_transform(documents)

    topic_info = model.get_topic_info()
    print(topic_info.head())

    return jsonify({
        "topics": [int(topic) for topic in topics]
    })

def monitor_parent():
    sys.stdin.buffer.read()
    os.kill(os.getpid(), signal.SIGTERM)

if __name__ == "__main__":
    server = create_server(app, host="127.0.0.1", port=0)
    print(f"PORT={server.effective_port}", flush=True)

    threading.Thread(
        target=monitor_parent,
        daemon=True
    ).start()

    server.run()