#!/usr/bin/env python3
"""Import generated eco_docs deck JSON files into a slides-backend Mongo db.

Usage: python3 import_decks.py <decks_dir> | mongosh --quiet mongodb://localhost:27017/slides_backend_getecosphere

Reads every .json file in decks_dir (one SlideDeck per file, matching the
slides domain model) and emits a mongosh script that upserts them by slug.
"""

import json
import os
import sys


def js_string(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def main():
    decks_dir = sys.argv[1]
    files = sorted(os.listdir(decks_dir))
    decks = []
    for fn in files:
        if not fn.endswith(".json"):
            continue
        with open(os.path.join(decks_dir, fn), encoding="utf-8") as f:
            decks.append(json.load(f))

    print("db.slide_decks.deleteMany({ slug: { $in: [", ",".join(js_string(d["slug"]) for d in decks), "] } });")
    for d in decks:
        doc = dict(d)
        doc["createdAt"] = {"$date": doc.get("createdAt", "2026-08-12T00:00:00Z")}
        doc["updatedAt"] = {"$date": doc.get("updatedAt", "2026-08-12T00:00:00Z")}
        text = json.dumps(doc, ensure_ascii=False)
        print(f"db.slide_decks.insertOne({text});")
    print(f"// inserted {len(decks)} decks")
    print("print('imported', db.slide_decks.countDocuments({slug:{$in:[" + ",".join(js_string(d["slug"]) for d in decks) + "]}}), 'eco-docs decks');")


if __name__ == "__main__":
    main()
