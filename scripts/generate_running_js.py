#!/usr/bin/env python3
"""
Convert running_data.json (produced by autorunner CLI) into js/data/running.js
for the personal website.
"""

import json
import sys
from pathlib import Path

SPORT_TYPE_EN = {
    "outdoor_run": "Outdoor Run",
    "indoor_run": "Indoor Run",
    "trail_run": "Trail Run",
    "track_run": "Track Run",
    "run": "Run",
}

SPORT_TYPE_ZH = {
    "outdoor_run": "户外跑步",
    "indoor_run": "室内跑步",
    "trail_run": "越野跑",
    "track_run": "操场跑",
    "run": "跑步",
}

RACE_NAME_ZH = {
    "5K": "5公里",
    "10K": "10公里",
    "Half Marathon": "半程马拉松",
    "Marathon": "全程马拉松",
}

PB_NAME_ZH = {
    "1K": "1公里",
    "3K": "3公里",
    "5K": "5公里",
    "10K": "10公里",
    "Half Marathon": "半程马拉松",
    "Marathon": "全程马拉松",
}


def format_distance(km: float) -> str:
    if km >= 1000:
        return f"{km:,.1f}"
    return f"{km:.2f}"


def format_duration_en(seconds: int) -> str:
    hours = seconds // 3600
    minutes = (seconds % 3600) // 60
    if hours > 0:
        return f"{hours}h {minutes}m"
    return f"{minutes}m"


def format_duration_zh(seconds: int) -> str:
    hours = seconds // 3600
    minutes = (seconds % 3600) // 60
    if hours > 0:
        return f"{hours}小时{minutes}分钟"
    return f"{minutes}分钟"


def format_activity_duration(seconds: int) -> str:
    hours = seconds // 3600
    minutes = (seconds % 3600) // 60
    secs = seconds % 60
    if hours > 0:
        return f"{hours}:{minutes:02d}:{secs:02d}"
    return f"{minutes}:{secs:02d}"


def format_race_time(seconds: int) -> str:
    hours = seconds // 3600
    minutes = (seconds % 3600) // 60
    secs = seconds % 60
    if hours > 0:
        return f"{hours}:{minutes:02d}:{secs:02d}"
    return f"{minutes}:{secs:02d}"


def build_dashboard_en(dashboard: dict | None) -> dict | None:
    if not dashboard:
        return None
    result = {}
    if dashboard.get("running_level") is not None:
        result["runningLevel"] = round(dashboard["running_level"], 1)
    scores = dashboard.get("scores", {})
    result["scores"] = {
        "aerobicEndurance": scores.get("aerobic_endurance"),
        "lactateThreshold": scores.get("lactate_threshold"),
        "anaerobicEndurance": scores.get("anaerobic_endurance"),
        "anaerobicCapacity": scores.get("anaerobic_capacity"),
    }
    result["scoresLabels"] = {
        "aerobicEndurance": "Aerobic Endurance",
        "lactateThreshold": "Lactate Threshold",
        "anaerobicEndurance": "Anaerobic Endurance",
        "anaerobicCapacity": "Anaerobic Capacity",
    }
    if dashboard.get("resting_hr"):
        result["restingHr"] = dashboard["resting_hr"]
    if dashboard.get("threshold_hr"):
        result["thresholdHr"] = dashboard["threshold_hr"]
    if dashboard.get("threshold_pace"):
        result["thresholdPace"] = f"{dashboard['threshold_pace']}/km"
    if dashboard.get("recovery_pct") is not None:
        result["recoveryPct"] = round(dashboard["recovery_pct"])
    preds = dashboard.get("race_predictions", [])
    if preds:
        result["racePredictions"] = [
            {
                "race": r["race"],
                "time": format_race_time(r["duration_seconds"]),
                "pace": f"{r['avg_pace']}/km",
            }
            for r in preds
        ]
    result["labels"] = {
        "runningLevel": "Running Level",
        "restingHr": "Resting HR",
        "thresholdHr": "Threshold HR",
        "thresholdPace": "Threshold Pace",
        "recoveryPct": "Recovery",
        "racePredictions": "Race Predictions",
        "personalBests": "Personal Bests",
        "race": "Race",
        "time": "Time",
        "pace": "Pace",
        "date": "Date",
    }
    return result


def build_dashboard_zh(dashboard: dict | None) -> dict | None:
    if not dashboard:
        return None
    result = {}
    if dashboard.get("running_level") is not None:
        result["runningLevel"] = round(dashboard["running_level"], 1)
    scores = dashboard.get("scores", {})
    result["scores"] = {
        "aerobicEndurance": scores.get("aerobic_endurance"),
        "lactateThreshold": scores.get("lactate_threshold"),
        "anaerobicEndurance": scores.get("anaerobic_endurance"),
        "anaerobicCapacity": scores.get("anaerobic_capacity"),
    }
    result["scoresLabels"] = {
        "aerobicEndurance": "有氧耐力",
        "lactateThreshold": "乳酸阈值",
        "anaerobicEndurance": "无氧耐力",
        "anaerobicCapacity": "无氧能力",
    }
    if dashboard.get("resting_hr"):
        result["restingHr"] = dashboard["resting_hr"]
    if dashboard.get("threshold_hr"):
        result["thresholdHr"] = dashboard["threshold_hr"]
    if dashboard.get("threshold_pace"):
        result["thresholdPace"] = f"{dashboard['threshold_pace']}/公里"
    if dashboard.get("recovery_pct") is not None:
        result["recoveryPct"] = round(dashboard["recovery_pct"])
    preds = dashboard.get("race_predictions", [])
    if preds:
        result["racePredictions"] = [
            {
                "race": RACE_NAME_ZH.get(r["race"], r["race"]),
                "time": format_race_time(r["duration_seconds"]),
                "pace": f"{r['avg_pace']}/公里",
            }
            for r in preds
        ]
    result["labels"] = {
        "runningLevel": "跑力",
        "restingHr": "静息心率",
        "thresholdHr": "乳酸阈心率",
        "thresholdPace": "乳酸阈配速",
        "recoveryPct": "恢复度",
        "racePredictions": "预测成绩",
        "personalBests": "最好成绩",
        "race": "项目",
        "time": "时间",
        "pace": "配速",
        "date": "日期",
    }
    return result


def build_pbs_en(pbs: list) -> list:
    return [
        {
            "distance": pb["distance"],
            "time": pb["time"],
            "pace": f"{pb['pace']}/km",
            "date": pb["date"],
        }
        for pb in pbs
    ]


def build_pbs_zh(pbs: list) -> list:
    return [
        {
            "distance": PB_NAME_ZH.get(pb["distance"], pb["distance"]),
            "time": pb["time"],
            "pace": f"{pb['pace']}/公里",
            "date": pb["date"],
        }
        for pb in pbs
    ]


def build_en(data: dict) -> dict:
    summary = data["summary"]
    result = {
        "sectionTitle": "Running",
        "summary": {
            "totalDistance": f"{format_distance(summary['total_distance_km'])} km",
            "totalRuns": summary["total_runs"],
            "avgPace": f"{summary['avg_pace']}/km",
            "totalDuration": format_duration_en(summary["total_duration_seconds"]),
            "longestRun": f"{format_distance(summary['longest_run_km'])} km",
        },
        "summaryLabels": {
            "totalDistance": "Total Distance",
            "totalRuns": "Total Runs",
            "avgPace": "Avg Pace",
            "totalDuration": "Total Time",
            "longestRun": "Longest Run",
        },
        "heatmap": data["heatmap"],
        "activities": [
            {
                "date": a["date"],
                "distance": f"{format_distance(a['distance_km'])} km",
                "duration": format_activity_duration(a["duration_seconds"]),
                "pace": f"{a['pace_per_km']}/km",
                "type": SPORT_TYPE_EN.get(a["sport_type"], "Run"),
                "calories": a["calories"],
            }
            for a in data["activities"]
        ],
        "activityLabels": {
            "date": "Date",
            "distance": "Distance",
            "duration": "Duration",
            "pace": "Pace",
            "type": "Type",
            "calories": "Calories",
        },
        "lastSynced": data["last_synced"],
    }
    dashboard = build_dashboard_en(data.get("dashboard"))
    if dashboard:
        result["dashboard"] = dashboard
    pbs = data.get("personal_bests", [])
    if pbs:
        result["personalBests"] = build_pbs_en(pbs)
    return result


def build_zh(data: dict) -> dict:
    summary = data["summary"]
    result = {
        "sectionTitle": "跑步",
        "summary": {
            "totalDistance": f"{format_distance(summary['total_distance_km'])} 公里",
            "totalRuns": summary["total_runs"],
            "avgPace": f"{summary['avg_pace']}/公里",
            "totalDuration": format_duration_zh(summary["total_duration_seconds"]),
            "longestRun": f"{format_distance(summary['longest_run_km'])} 公里",
        },
        "summaryLabels": {
            "totalDistance": "总里程",
            "totalRuns": "总次数",
            "avgPace": "平均配速",
            "totalDuration": "总时长",
            "longestRun": "最长跑",
        },
        "heatmap": data["heatmap"],
        "activities": [
            {
                "date": a["date"],
                "distance": f"{format_distance(a['distance_km'])} 公里",
                "duration": format_activity_duration(a["duration_seconds"]),
                "pace": f"{a['pace_per_km']}/公里",
                "type": SPORT_TYPE_ZH.get(a["sport_type"], "跑步"),
                "calories": a["calories"],
            }
            for a in data["activities"]
        ],
        "activityLabels": {
            "date": "日期",
            "distance": "距离",
            "duration": "时长",
            "pace": "配速",
            "type": "类型",
            "calories": "卡路里",
        },
        "lastSynced": data["last_synced"],
    }
    dashboard = build_dashboard_zh(data.get("dashboard"))
    if dashboard:
        result["dashboard"] = dashboard
    pbs = data.get("personal_bests", [])
    if pbs:
        result["personalBests"] = build_pbs_zh(pbs)
    return result


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <running_data.json> <output_running.js>")
        sys.exit(1)

    input_path = Path(sys.argv[1])
    output_path = Path(sys.argv[2])

    with open(input_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    running_data = {
        "en": build_en(data),
        "zh": build_zh(data),
    }

    js_content = "const runningData = " + json.dumps(running_data, ensure_ascii=False, indent=2) + ";\n"

    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        f.write(js_content)

    print(f"Generated {output_path}")


if __name__ == "__main__":
    main()
