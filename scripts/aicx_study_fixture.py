#!/usr/bin/env python3
"""AICX study bot fixture quiz generator, grader, and Slack smoke runner.

This runner is intentionally local-only: it reads manual schedule/topic fixtures,
does not read PDF text, and only calls Slack when explicitly run in live mode.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
import re
import threading
import time
from dataclasses import dataclass
from datetime import date, datetime
from pathlib import Path
from typing import Any
from zoneinfo import ZoneInfo


CHOICES = ("A", "B", "C", "D")
DEFAULT_WEAK_TOPIC_IDS = {"TOPIC-ORG-KPI", "TOPIC-5D"}
DEFAULT_WEAK_TOPIC_THRESHOLD = 0.8
AICX_EXAMPLE_DIR = Path("docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot")
DEFAULT_STUDY_SCHEDULE_PATH = AICX_EXAMPLE_DIR / "study_schedule.json"
DEFAULT_TOPIC_MAP_PATH = AICX_EXAMPLE_DIR / "topic_map.json"
DEFAULT_QUESTION_BANK_PATH = AICX_EXAMPLE_DIR / "question_bank.fixture.json"
SLACK_TOKEN_ENV = "SLACK_BOT_TOKEN"
SLACK_CHANNEL_ENV = "SLACK_CHANNEL_ID"
SLACK_APP_TOKEN_ENV = "SLACK_APP_TOKEN"
DAILY_RUN_STATE_SCHEMA_VERSION = "aicx.daily_run_state.v0"
MAINTENANCE_RECEIPT_SCHEMA_VERSION = "aicx.maintenance_receipt.v0"
SOCKET_REPLY_LISTEN_SUCCESS_STATUS = "received_and_graded"
SOCKET_REPLY_LISTEN_SENT_STATUS = "received_graded_and_sent"
SOCKET_REPLY_LISTEN_INVALID_SENT_STATUS = "received_invalid_reply_and_sent"
SOCKET_REPLY_LISTEN_DUPLICATE_STATUS = "duplicate_reply_skipped"
SOCKET_REPLY_LISTEN_SUCCESS_STATUSES = {
    SOCKET_REPLY_LISTEN_SUCCESS_STATUS,
    SOCKET_REPLY_LISTEN_SENT_STATUS,
    SOCKET_REPLY_LISTEN_INVALID_SENT_STATUS,
    SOCKET_REPLY_LISTEN_DUPLICATE_STATUS,
}
SLACK_THREAD_POLL_NO_REPLY_STATUS = "no_reply_found"
SLACK_THREAD_POLL_GRADED_STATUS = "reply_found_graded_and_sent"
SLACK_THREAD_POLL_INVALID_STATUS = "invalid_reply_found"
SLACK_THREAD_POLL_DUPLICATE_STATUS = "duplicate_reply_skipped"
SLACK_THREAD_POLL_FAILURE_STATUSES = {
    "blocked_missing_env_or_sdk",
    "poll_failed",
    "rate_limited",
    "thread_not_found",
}
SLACK_THREAD_POLL_SUCCESS_STATUSES = {
    SLACK_THREAD_POLL_NO_REPLY_STATUS,
    SLACK_THREAD_POLL_GRADED_STATUS,
    SLACK_THREAD_POLL_INVALID_STATUS,
    SLACK_THREAD_POLL_DUPLICATE_STATUS,
}
ANSWER_TOKEN_PATTERN = re.compile(r"(?:^|[\s,;])(?:Q)?(\d{1,3})\s*[:.\-)]\s*([A-Da-d])\b")


@dataclass(frozen=True)
class PageRange:
    start: int
    end: int

    def as_json(self) -> dict[str, int]:
        return {"from": self.start, "to": self.end}


def load_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return value


def load_json_if_exists(path: Path) -> dict[str, Any] | None:
    if not path.exists():
        return None
    return load_json(path)


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def write_text(path: Path, value: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(value, encoding="utf-8")


def load_env_file(path: Path | None) -> list[str]:
    if path is None or not path.exists():
        return []

    loaded = []
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, raw_value = line.split("=", 1)
        key = key.strip()
        value = raw_value.strip().strip('"').strip("'")
        if key and value and key not in os.environ:
            os.environ[key] = value
            loaded.append(key)
    return loaded


def module_available(name: str) -> bool:
    try:
        return importlib.util.find_spec(name) is not None
    except ModuleNotFoundError:
        return False


def parse_runner_now(now_value: str | None, timezone_name: str) -> datetime:
    timezone = ZoneInfo(timezone_name)
    if not now_value:
        return datetime.now(timezone)
    parsed = datetime.fromisoformat(now_value.replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        return parsed.replace(tzinfo=timezone)
    return parsed.astimezone(timezone)


def scheduled_datetime(target_date: str, delivery_time: str, timezone_name: str) -> datetime:
    parsed_date = date.fromisoformat(target_date)
    hour_text, minute_text = delivery_time.split(":", 1)
    return datetime(
        parsed_date.year,
        parsed_date.month,
        parsed_date.day,
        int(hour_text),
        int(minute_text),
        tzinfo=ZoneInfo(timezone_name),
    )


def daily_dispatch_is_due(target_date: str, delivery_time: str, timezone_name: str, now_value: str | None) -> bool:
    return parse_runner_now(now_value, timezone_name) >= scheduled_datetime(
        target_date,
        delivery_time,
        timezone_name,
    )


def classify_daily_dispatch_timing(
    target_date: str,
    delivery_time: str,
    timezone_name: str,
    now_value: str | None,
    allow_late: bool = False,
    late_window_hours: int | None = None,
) -> dict[str, Any]:
    now = parse_runner_now(now_value, timezone_name)
    scheduled_at = scheduled_datetime(target_date, delivery_time, timezone_name)
    late_by_seconds = int((now - scheduled_at).total_seconds())
    if late_by_seconds < 0:
        return {
            "status": "not_due",
            "scheduled_at": scheduled_at.isoformat(),
            "scheduled_time": delivery_time,
            "late_by_seconds": 0,
            "late_window_hours": late_window_hours,
        }
    if allow_late and late_by_seconds > 0:
        if late_window_hours is not None and late_by_seconds > late_window_hours * 3600:
            return {
                "status": "late_window_expired",
                "scheduled_at": scheduled_at.isoformat(),
                "scheduled_time": delivery_time,
                "late_by_seconds": late_by_seconds,
                "late_window_hours": late_window_hours,
            }
        return {
            "status": "late_dispatched",
            "scheduled_at": scheduled_at.isoformat(),
            "scheduled_time": delivery_time,
            "late_by_seconds": late_by_seconds,
            "late_window_hours": late_window_hours,
        }
    return {
        "status": "due",
        "scheduled_at": scheduled_at.isoformat(),
        "scheduled_time": delivery_time,
        "late_by_seconds": max(0, late_by_seconds),
        "late_window_hours": late_window_hours,
    }


def render_dispatch_notice(dispatch_timing: dict[str, Any] | None) -> str | None:
    if not dispatch_timing or dispatch_timing.get("status") != "late_dispatched":
        return None
    late_minutes = max(1, round(int(dispatch_timing.get("late_by_seconds") or 0) / 60))
    return f"遅れて配信: {dispatch_timing.get('scheduled_time', '04:30')}予定から約{late_minutes}分後の配信です。"


def parse_page_range(value: dict[str, Any]) -> PageRange:
    return PageRange(int(value["from"]), int(value["to"]))


def overlap(left: PageRange, right: PageRange) -> PageRange | None:
    start = max(left.start, right.start)
    end = min(left.end, right.end)
    if start > end:
        return None
    return PageRange(start, end)


def page_range_label(ranges: list[PageRange]) -> str:
    return ", ".join(f"p.{item.start}-{item.end}" for item in ranges)


def json_page_range_label(ranges: list[dict[str, Any]]) -> str:
    return page_range_label([parse_page_range(item) for item in ranges])


def find_study_window(schedule: dict[str, Any], target_date: str) -> dict[str, Any]:
    parsed = date.fromisoformat(target_date)
    for week in schedule.get("weeks", []):
        start = date.fromisoformat(week["date_from"])
        end = date.fromisoformat(week["date_to"])
        if start <= parsed <= end:
            return week
    raise ValueError(f"no study schedule week covers {target_date}")


def focus_matches(topic: dict[str, Any], focus_topics: list[str]) -> bool:
    text = f"{topic.get('topic_id', '')} {topic.get('label', '')} {topic.get('question_style', '')}"
    return any(focus and focus in text for focus in focus_topics)


def resolve_topics(study_window: dict[str, Any], topic_map: dict[str, Any]) -> list[dict[str, Any]]:
    study_ranges = [parse_page_range(item) for item in study_window.get("study_pages", [])]
    focus_topics = [str(item) for item in study_window.get("focus_topics", [])]
    resolved: list[dict[str, Any]] = []

    for topic in topic_map.get("topics", []):
        raw_ranges = [parse_page_range(item) for item in topic.get("page_ranges", [])]
        matching_ranges: list[PageRange] = []

        if study_ranges:
            for raw_range in raw_ranges:
                for study_range in study_ranges:
                    matched = overlap(raw_range, study_range)
                    if matched:
                        matching_ranges.append(matched)
        elif focus_matches(topic, focus_topics):
            matching_ranges.extend(raw_ranges)

        if matching_ranges:
            resolved.append(
                {
                    "topic_id": topic["topic_id"],
                    "label": topic["label"],
                    "question_style": topic.get("question_style", "実践判断問題"),
                    "page_ranges": [item.as_json() for item in matching_ranges],
                    "_sort_page": min(item.start for item in matching_ranges),
                }
            )

    if not resolved:
        raise ValueError(f"no topic_map entries match {study_window['week_id']}")

    resolved.sort(key=lambda item: (int(item["_sort_page"]), str(item["topic_id"])))
    for item in resolved:
        item.pop("_sort_page", None)
    return resolved


def rotated_choices(correct_text: str, distractors: list[str], index: int) -> tuple[list[dict[str, str]], str]:
    ordered_texts = [correct_text, *distractors]
    shift = index % len(CHOICES)
    rotated = ordered_texts[shift:] + ordered_texts[:shift]
    correct_choice = CHOICES[rotated.index(correct_text)]
    choices = [
        {
            "choice": choice,
            "text": text,
        }
        for choice, text in zip(CHOICES, rotated)
    ]
    return choices, correct_choice


def build_question(target_date: str, topic: dict[str, Any], index: int) -> dict[str, Any]:
    ranges = [parse_page_range(item) for item in topic["page_ranges"]]
    label = topic["label"]
    correct_text = f"{label} の判断基準を、業務目的・制約・測定可能な成功条件に結びつける"
    distractors = [
        "使いたいツールを先に固定し、業務条件は導入後に合わせる",
        "例外処理、法務、運用責任を PoC 後まで確認しない",
        "正答率だけを見て、弱点トピックや推奨ページとの対応を残さない",
    ]
    choices, correct_choice = rotated_choices(correct_text, distractors, index)
    return {
        "question_id": f"Q-ASB-{target_date}-{index + 1:03d}",
        "topic_id": topic["topic_id"],
        "topic_label": label,
        "page_ranges": [item.as_json() for item in ranges],
        "source": "manual_fixture_topic_map",
        "difficulty": "practical",
        "prompt": (
            f"{label} の観点で {page_range_label(ranges)} を復習しています。"
            "AIエージェント導入の現場判断として最も適切なものはどれですか。"
        ),
        "choices": choices,
        "correct_choice": correct_choice,
        "rationale": (
            "PoC-1 fixture では PDF 本文を使わず、topic_map のページ範囲と"
            "実務判断の対応を確認するため、目的・制約・成功条件への接続を正解にします。"
        ),
    }


def bank_question_to_quiz_question(
    topic: dict[str, Any],
    bank_question: dict[str, Any],
) -> dict[str, Any]:
    return {
        "question_id": bank_question["question_id"],
        "topic_id": bank_question["topic_id"],
        "topic_label": topic["label"],
        "page_ranges": bank_question["page_ranges"],
        "source": "question_bank_fixture",
        "difficulty": "practical",
        "prompt": bank_question["prompt"],
        "choices": bank_question["choices"],
        "correct_choice": bank_question["correct_choice"],
        "rationale": bank_question["rationale"],
        "scenario_tags": bank_question.get("scenario_tags", []),
    }


def select_question_bank_questions(
    topics: list[dict[str, Any]],
    question_bank: dict[str, Any] | None,
) -> list[dict[str, Any]]:
    if not question_bank:
        return []

    questions_by_topic: dict[str, list[dict[str, Any]]] = {}
    for question in question_bank.get("questions", []):
        questions_by_topic.setdefault(question["topic_id"], []).append(question)
    for questions in questions_by_topic.values():
        questions.sort(key=lambda item: item["question_id"])

    selected = []
    for topic in topics:
        for question in questions_by_topic.get(topic["topic_id"], []):
            selected.append(bank_question_to_quiz_question(topic, question))
    return selected


def question_bank_by_topic(question_bank: dict[str, Any] | None) -> dict[str, list[dict[str, Any]]]:
    if not question_bank:
        return {}

    questions_by_topic: dict[str, list[dict[str, Any]]] = {}
    for question in question_bank.get("questions", []):
        questions_by_topic.setdefault(question["topic_id"], []).append(question)
    for questions in questions_by_topic.values():
        questions.sort(key=lambda item: item["question_id"])
    return questions_by_topic


def generate_quiz_set(
    schedule: dict[str, Any],
    topic_map: dict[str, Any],
    target_date: str,
    question_count: int | None = None,
    question_bank: dict[str, Any] | None = None,
) -> dict[str, Any]:
    study_window = find_study_window(schedule, target_date)
    topics = resolve_topics(study_window, topic_map)
    count = question_count or int(study_window.get("daily_quiz", {}).get("question_count") or schedule["default_question_count"])

    questions = select_question_bank_questions(topics, question_bank)[:count]
    for index in range(len(questions), count):
        questions.append(build_question(target_date, topics[index % len(topics)], index))

    return build_quiz_set_payload(schedule, study_window, topics, target_date, questions)


def build_quiz_set_payload(
    schedule: dict[str, Any],
    study_window: dict[str, Any],
    topics: list[dict[str, Any]],
    target_date: str,
    questions: list[dict[str, Any]],
) -> dict[str, Any]:
    return {
        "schema_version": "aicx.quiz_set.v0",
        "quiz_set_id": f"QUIZ-ASB-{target_date}",
        "program_id": schedule["program_id"],
        "epic_id": schedule["epic_id"],
        "generated_for_date": target_date,
        "timezone": schedule.get("timezone", "Asia/Tokyo"),
        "delivery_time": schedule.get("default_daily_delivery_time", "04:30"),
        "source_mode": "manual_fixture",
        "pdf_ingest_used": False,
        "slack_used": False,
        "line_used": False,
        "answer_format": "multiple_choice_a_d",
        "study_window": {
            "week_id": study_window["week_id"],
            "date_from": study_window["date_from"],
            "date_to": study_window["date_to"],
            "study_pages": study_window.get("study_pages", []),
            "focus_topics": study_window.get("focus_topics", []),
        },
        "topic_scope": [
            {
                "topic_id": topic["topic_id"],
                "label": topic["label"],
                "page_ranges": topic["page_ranges"],
            }
            for topic in topics
        ],
        "question_count": len(questions),
        "questions": questions,
    }


def validate_adaptive_plan_for_dispatch(
    adaptive_plan: dict[str, Any],
    target_date: str,
    question_count: int,
    study_window: dict[str, Any],
    topics: list[dict[str, Any]],
) -> None:
    if adaptive_plan.get("schema_version") != "aicx.adaptive_plan.v0":
        raise ValueError("adaptive_plan schema_version must be aicx.adaptive_plan.v0")

    plan_date = adaptive_plan.get("next_quiz_date")
    if plan_date != target_date:
        raise ValueError(f"adaptive_plan next_quiz_date {plan_date} does not match dispatch date {target_date}")

    plan_count = int(adaptive_plan.get("question_count", -1))
    if plan_count != question_count:
        raise ValueError(f"adaptive_plan question_count {plan_count} does not match dispatch question_count {question_count}")

    plan_window = adaptive_plan.get("study_window", {})
    if plan_window.get("week_id") != study_window.get("week_id"):
        raise ValueError(
            f"adaptive_plan study_window {plan_window.get('week_id')} does not match dispatch study_window {study_window.get('week_id')}"
        )

    allocations = adaptive_plan.get("topic_allocations", [])
    if not allocations:
        raise ValueError("adaptive_plan topic_allocations must not be empty")

    allocation_topic_ids = [allocation["topic_id"] for allocation in allocations]
    duplicates = sorted(
        topic_id for topic_id in set(allocation_topic_ids)
        if allocation_topic_ids.count(topic_id) > 1
    )
    if duplicates:
        raise ValueError(f"adaptive_plan topic_allocations contains duplicate topic_id values: {duplicates}")

    topic_scope_ids = {topic["topic_id"] for topic in topics}
    unknown_topics = sorted(set(allocation_topic_ids) - topic_scope_ids)
    if unknown_topics:
        raise ValueError(f"adaptive_plan topic_allocations contains topics outside dispatch scope: {unknown_topics}")

    allocation_total = sum(int(allocation["question_count"]) for allocation in allocations)
    if allocation_total != plan_count:
        raise ValueError(
            f"adaptive_plan topic_allocations total {allocation_total} does not match question_count {plan_count}"
        )


def generate_quiz_set_from_adaptive_plan(
    schedule: dict[str, Any],
    topic_map: dict[str, Any],
    target_date: str,
    adaptive_plan: dict[str, Any],
    question_count: int | None = None,
    question_bank: dict[str, Any] | None = None,
) -> dict[str, Any]:
    study_window = find_study_window(schedule, target_date)
    topics = resolve_topics(study_window, topic_map)
    count = question_count or int(study_window.get("daily_quiz", {}).get("question_count") or schedule["default_question_count"])
    validate_adaptive_plan_for_dispatch(adaptive_plan, target_date, count, study_window, topics)
    questions = select_adaptive_questions(target_date, adaptive_plan["topic_allocations"], topics, question_bank)
    if len(questions) != count:
        raise ValueError(f"adaptive_plan selected {len(questions)} questions but dispatch expected {count}")
    return build_quiz_set_payload(schedule, study_window, topics, target_date, questions)


def aggregate_topic_results(grading_reports: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    totals: dict[str, dict[str, Any]] = {}
    for report in grading_reports:
        report_id = str(report.get("grading_report_id", ""))
        for topic in report.get("topic_results", []):
            topic_id = topic["topic_id"]
            bucket = totals.setdefault(
                topic_id,
                {
                    "topic_id": topic_id,
                    "label": topic["label"],
                    "page_ranges": topic["page_ranges"],
                    "total": 0,
                    "correct": 0,
                    "source_grading_report_ids": [],
                },
            )
            bucket["total"] += int(topic["total"])
            bucket["correct"] += int(topic["correct"])
            if report_id and report_id not in bucket["source_grading_report_ids"]:
                bucket["source_grading_report_ids"].append(report_id)
    for topic in totals.values():
        topic["accuracy"] = round(topic["correct"] / topic["total"], 4) if topic["total"] else None
    return totals


def build_topic_result_for_plan(
    topic: dict[str, Any],
    aggregate: dict[str, Any] | None,
    weak_topic_threshold: float,
) -> dict[str, Any]:
    if aggregate:
        total = int(aggregate["total"])
        correct = int(aggregate["correct"])
        accuracy = aggregate["accuracy"]
        source_ids = aggregate["source_grading_report_ids"]
    else:
        total = 0
        correct = 0
        accuracy = None
        source_ids = []
    return {
        "topic_id": topic["topic_id"],
        "label": topic["label"],
        "page_ranges": topic["page_ranges"],
        "total": total,
        "correct": correct,
        "accuracy": accuracy,
        "is_weak": accuracy is not None and accuracy < weak_topic_threshold,
        "source_grading_report_ids": source_ids,
    }


def allocate_adaptive_questions(
    topics: list[dict[str, Any]],
    topic_results: list[dict[str, Any]],
    question_count: int,
) -> list[dict[str, Any]]:
    if question_count < 1:
        raise ValueError("question_count must be at least 1")

    result_by_id = {result["topic_id"]: result for result in topic_results}
    allocation_counts = {topic["topic_id"]: 0 for topic in topics}

    if question_count >= len(topics):
        for topic in topics:
            allocation_counts[topic["topic_id"]] = 1
        remaining = question_count - len(topics)
    else:
        remaining = question_count

    weak_topics = [
        result for result in topic_results
        if result["is_weak"]
    ]
    weak_topics.sort(key=lambda item: (item["accuracy"], item["topic_id"]))
    priority_topics = weak_topics or sorted(
        topic_results,
        key=lambda item: (
            1.0 if item["accuracy"] is None else item["accuracy"],
            item["topic_id"],
        ),
    )

    index = 0
    while remaining:
        topic_id = priority_topics[index % len(priority_topics)]["topic_id"]
        allocation_counts[topic_id] += 1
        remaining -= 1
        index += 1

    allocations = []
    for topic in topics:
        result = result_by_id[topic["topic_id"]]
        count = allocation_counts[topic["topic_id"]]
        if count == 0:
            basis = "out_of_daily_limit"
        elif result["is_weak"]:
            basis = "weak_topic_boost"
        elif result["accuracy"] is None:
            basis = "no_history_maintenance"
        else:
            basis = "maintenance"
        allocations.append(
            {
                "topic_id": topic["topic_id"],
                "label": topic["label"],
                "page_ranges": topic["page_ranges"],
                "question_count": count,
                "basis": basis,
                "accuracy": result["accuracy"],
            }
        )
    return allocations


def select_adaptive_questions(
    target_date: str,
    allocations: list[dict[str, Any]],
    topics: list[dict[str, Any]],
    question_bank: dict[str, Any] | None,
) -> list[dict[str, Any]]:
    topic_by_id = {topic["topic_id"]: topic for topic in topics}
    bank_by_topic = question_bank_by_topic(question_bank)
    selected: list[dict[str, Any]] = []

    for allocation in allocations:
        topic_id = allocation["topic_id"]
        topic = topic_by_id[topic_id]
        bank_questions = bank_by_topic.get(topic_id, [])
        for local_index in range(allocation["question_count"]):
            if local_index < len(bank_questions):
                selected.append(bank_question_to_quiz_question(topic, bank_questions[local_index]))
            else:
                selected.append(build_question(target_date, topic, len(selected)))
    return selected


def build_adaptive_plan(
    schedule: dict[str, Any],
    topic_map: dict[str, Any],
    question_bank: dict[str, Any] | None,
    grading_reports: list[dict[str, Any]],
    target_date: str,
    question_count: int | None = None,
    weak_topic_threshold: float = DEFAULT_WEAK_TOPIC_THRESHOLD,
) -> dict[str, Any]:
    study_window = find_study_window(schedule, target_date)
    topics = resolve_topics(study_window, topic_map)
    count = question_count or int(study_window.get("daily_quiz", {}).get("question_count") or schedule["default_question_count"])
    aggregated = aggregate_topic_results(grading_reports)
    topic_results = [
        build_topic_result_for_plan(topic, aggregated.get(topic["topic_id"]), weak_topic_threshold)
        for topic in topics
    ]
    allocations = allocate_adaptive_questions(topics, topic_results, count)
    questions = select_adaptive_questions(target_date, allocations, topics, question_bank)
    source_report_ids = [
        report["grading_report_id"]
        for report in grading_reports
        if report.get("grading_report_id")
    ]
    weak_topics = [
        {
            "topic_id": result["topic_id"],
            "label": result["label"],
            "page_ranges": result["page_ranges"],
            "accuracy": result["accuracy"],
            "recommended_question_count": next(
                allocation["question_count"]
                for allocation in allocations
                if allocation["topic_id"] == result["topic_id"]
            ),
        }
        for result in topic_results
        if result["is_weak"]
    ]
    question_selection = []
    for index, question in enumerate(questions, start=1):
        source = question["source"]
        question_selection.append(
            {
                "question_number": index,
                "question_id": question["question_id"],
                "topic_id": question["topic_id"],
                "source": source,
                "selection_reason": (
                    "question_bank_priority"
                    if source == "question_bank_fixture"
                    else "topic_map_fallback_after_bank_exhausted"
                ),
            }
        )

    return {
        "schema_version": "aicx.adaptive_plan.v0",
        "adaptive_plan_id": f"ADAPTIVE-ASB-{target_date}",
        "program_id": schedule["program_id"],
        "epic_id": schedule["epic_id"],
        "generated_for_date": target_date,
        "next_quiz_date": target_date,
        "timezone": schedule.get("timezone", "Asia/Tokyo"),
        "source_grading_report_ids": source_report_ids,
        "weak_topic_threshold": weak_topic_threshold,
        "question_count": count,
        "study_window": {
            "week_id": study_window["week_id"],
            "date_from": study_window["date_from"],
            "date_to": study_window["date_to"],
            "study_pages": study_window.get("study_pages", []),
            "focus_topics": study_window.get("focus_topics", []),
        },
        "topic_results": topic_results,
        "weak_topics": weak_topics,
        "topic_allocations": allocations,
        "question_selection": question_selection,
        "selection_policy": {
            "weak_topic_rule": "topic accuracy below threshold receives extra allocation",
            "question_source_priority": ["question_bank_fixture", "manual_fixture_topic_map"],
            "pdf_ingest_used": False,
            "llm_generation_used": False,
            "slack_used": False,
        },
        "next_actions": [
            "翌日の10問は topic_allocations の配分で生成する",
            "question_bank_fixture を優先し、不足分だけ topic_map fallback を使う",
            "80%未満のtopicは次回採点後も継続して配分を見直す",
        ],
    }


def build_quiz_prompt(quiz_set: dict[str, Any], source_quiz_set_path: str | None = None) -> dict[str, Any]:
    study_ranges = [parse_page_range(item) for item in quiz_set["study_window"]["study_pages"]]
    questions = []
    for index, question in enumerate(quiz_set["questions"], start=1):
        questions.append(
            {
                "question_number": index,
                "question_id": question["question_id"],
                "topic_id": question["topic_id"],
                "topic_label": question["topic_label"],
                "page_ranges": question["page_ranges"],
                "prompt": question["prompt"],
                "choices": question["choices"],
            }
        )

    return {
        "schema_version": "aicx.quiz_prompt.v0",
        "quiz_prompt_id": f"PROMPT-{quiz_set['quiz_set_id']}",
        "quiz_set_id": quiz_set["quiz_set_id"],
        "generated_for_date": quiz_set["generated_for_date"],
        "timezone": quiz_set["timezone"],
        "delivery_time": quiz_set["delivery_time"],
        "source_quiz_set_path": source_quiz_set_path,
        "answer_key_included": False,
        "rationale_included": False,
        "answer_format": quiz_set["answer_format"],
        "study_window": quiz_set["study_window"],
        "study_range_label": page_range_label(study_ranges),
        "question_count": quiz_set["question_count"],
        "questions": questions,
    }


def render_quiz_prompt_markdown(quiz_prompt: dict[str, Any]) -> str:
    lines = [
        f"# AICX朝トレ {quiz_prompt['generated_for_date']}",
        "",
        f"- 範囲: {quiz_prompt['study_range_label']}",
        f"- 問題数: {quiz_prompt['question_count']}問",
        "- 回答形式: `1:B 2:C ...`",
        "- このartifactには正答・解説を含めません。",
        "",
        "## 問題",
        "",
    ]

    for question in quiz_prompt["questions"]:
        lines.extend(
            [
                f"### Q{question['question_number']}",
                "",
                question["prompt"],
                "",
            ]
        )
        for choice in question["choices"]:
            lines.append(f"{choice['choice']}. {choice['text']}")
        lines.append("")

    return "\n".join(lines).rstrip() + "\n"


def wrong_choice(correct_choice: str) -> str:
    index = CHOICES.index(correct_choice)
    return CHOICES[(index + 1) % len(CHOICES)]


def generate_fixture_submission(quiz_set: dict[str, Any]) -> dict[str, Any]:
    topic_seen: dict[str, int] = {}
    answers = []
    for question in quiz_set["questions"]:
        topic_id = question["topic_id"]
        topic_seen[topic_id] = topic_seen.get(topic_id, 0) + 1
        selected = question["correct_choice"]
        if topic_id in DEFAULT_WEAK_TOPIC_IDS and topic_seen[topic_id] <= 3:
            selected = wrong_choice(question["correct_choice"])
        answers.append(
            {
                "question_id": question["question_id"],
                "selected_choice": selected,
            }
        )

    return {
        "schema_version": "aicx.answer_submission.v0",
        "submission_id": f"SUB-{quiz_set['quiz_set_id']}-FIXTURE",
        "quiz_set_id": quiz_set["quiz_set_id"],
        "submitted_for_date": quiz_set["generated_for_date"],
        "source": "fixture_answer_grading",
        "answers": answers,
    }


def render_answer_submission_text(submission: dict[str, Any]) -> str:
    return " ".join(
        f"{index}:{answer['selected_choice']}"
        for index, answer in enumerate(submission["answers"], start=1)
    )


def build_slack_reply_event_fixture(quiz_set: dict[str, Any], submission: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": "aicx.slack_reply_event_fixture.v0",
        "reply_event_id": f"SLACKREPLY-{quiz_set['quiz_set_id']}-FIXTURE",
        "quiz_set_id": quiz_set["quiz_set_id"],
        "message_id": f"SLACKMSG-{quiz_set['quiz_set_id']}",
        "delivery_channel": "slack",
        "channel_id": "C_FIXTURE",
        "user_id": "U_FIXTURE",
        "thread_ts": "1782074738.865009",
        "event_ts": "1782074800.000000",
        "text": render_answer_submission_text(submission),
        "source": "manual_fixture",
    }


def build_socket_mode_payload_fixture(quiz_set: dict[str, Any], submission: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": "aicx.slack_socket_mode_payload_fixture.v0",
        "envelope_id": f"ENV-{quiz_set['quiz_set_id']}-FIXTURE",
        "type": "events_api",
        "accepts_response_payload": False,
        "payload": {
            "event_id": f"EV-{quiz_set['quiz_set_id']}-FIXTURE",
            "team_id": "T_FIXTURE",
            "event": {
                "type": "message",
                "channel": "C_FIXTURE",
                "user": "U_FIXTURE",
                "text": render_answer_submission_text(submission),
                "ts": "1782074800.000000",
                "thread_ts": "1782074738.865009",
            },
        },
        "source": "manual_socket_mode_fixture",
    }


def build_slack_reply_event_from_socket_payload(
    quiz_set: dict[str, Any],
    payload: dict[str, Any],
    expected_channel_id: str | None = None,
    expected_thread_ts: str | None = None,
    socket_envelope_id: str | None = None,
) -> dict[str, Any] | None:
    event = payload.get("event", {})
    if event.get("type") != "message":
        return None
    if event.get("subtype") is not None:
        return None
    if event.get("bot_id"):
        return None

    thread_ts = str(event.get("thread_ts") or "")
    event_ts = str(event.get("ts") or "")
    if not thread_ts or not event_ts or thread_ts == event_ts:
        return None

    channel_id = str(event.get("channel") or "")
    if expected_channel_id and channel_id != expected_channel_id:
        return None
    if expected_thread_ts and thread_ts != expected_thread_ts:
        return None

    event_id = str(payload.get("event_id") or event_ts).replace(".", "_")
    reply_event = {
        "schema_version": "aicx.slack_reply_event_fixture.v0",
        "reply_event_id": f"SLACKREPLY-{quiz_set['quiz_set_id']}-{event_id}",
        "quiz_set_id": quiz_set["quiz_set_id"],
        "message_id": f"SLACKMSG-{quiz_set['quiz_set_id']}",
        "delivery_channel": "slack",
        "channel_id": channel_id,
        "user_id": str(event.get("user") or ""),
        "thread_ts": thread_ts,
        "event_ts": event_ts,
        "text": str(event.get("text") or ""),
        "source": "socket_mode_event",
    }
    if socket_envelope_id:
        reply_event["socket_envelope_id"] = socket_envelope_id
    return reply_event


def build_slack_reply_event_from_thread_message(
    quiz_set: dict[str, Any],
    message: dict[str, Any],
    channel_id: str,
    thread_ts: str,
) -> dict[str, Any] | None:
    if message.get("subtype") is not None:
        return None
    if message.get("bot_id"):
        return None

    event_ts = str(message.get("ts") or "")
    if not event_ts or event_ts == thread_ts:
        return None

    message_thread_ts = str(message.get("thread_ts") or thread_ts)
    if message_thread_ts != thread_ts:
        return None

    user_id = str(message.get("user") or "")
    text = str(message.get("text") or "")
    if not user_id or not text:
        return None

    raw_event_id = str(message.get("client_msg_id") or event_ts).replace(".", "_").replace("-", "_")
    return {
        "schema_version": "aicx.slack_reply_event_fixture.v0",
        "reply_event_id": f"SLACKREPLY-{quiz_set['quiz_set_id']}-{raw_event_id}",
        "quiz_set_id": quiz_set["quiz_set_id"],
        "message_id": f"SLACKMSG-{quiz_set['quiz_set_id']}",
        "delivery_channel": "slack",
        "channel_id": channel_id,
        "user_id": user_id,
        "thread_ts": thread_ts,
        "event_ts": event_ts,
        "text": text,
        "source": "thread_poll_event",
    }


def select_unprocessed_thread_reply_event(
    quiz_set: dict[str, Any],
    messages: list[dict[str, Any]],
    channel_id: str,
    thread_ts: str,
    processed_reply_event_ids: list[str],
) -> tuple[dict[str, Any] | None, int]:
    candidates = []
    processed = set(processed_reply_event_ids)
    for message in sorted(messages, key=lambda item: str(item.get("ts") or "")):
        reply_event = build_slack_reply_event_from_thread_message(
            quiz_set,
            message,
            channel_id,
            thread_ts,
        )
        if reply_event is None:
            continue
        candidates.append(reply_event)
        if reply_event["reply_event_id"] not in processed:
            return reply_event, len(candidates)
    return None, len(candidates)


def parse_slack_answer_text(text: str) -> tuple[dict[int, str], list[int]]:
    answers: dict[int, str] = {}
    duplicates: list[int] = []
    for match in ANSWER_TOKEN_PATTERN.finditer(text):
        question_number = int(match.group(1))
        if question_number in answers:
            duplicates.append(question_number)
        answers[question_number] = match.group(2).upper()
    return answers, duplicates


def build_submission_from_slack_reply(quiz_set: dict[str, Any], reply_event: dict[str, Any]) -> dict[str, Any]:
    if reply_event.get("quiz_set_id") != quiz_set["quiz_set_id"]:
        raise ValueError(
            f"reply quiz_set_id {reply_event.get('quiz_set_id')} does not match {quiz_set['quiz_set_id']}"
        )

    parsed_answers, duplicates = parse_slack_answer_text(reply_event.get("text", ""))
    expected_numbers = set(range(1, len(quiz_set["questions"]) + 1))
    parsed_numbers = set(parsed_answers)
    missing = sorted(expected_numbers - parsed_numbers)
    extra = sorted(parsed_numbers - expected_numbers)
    if duplicates or missing or extra:
        raise ValueError(f"invalid Slack answer text; duplicates={duplicates}, missing={missing}, extra={extra}")

    answers = []
    for index, question in enumerate(quiz_set["questions"], start=1):
        answers.append(
            {
                "question_id": question["question_id"],
                "selected_choice": parsed_answers[index],
            }
        )

    source = reply_event.get("source")
    is_socket_mode = source == "socket_mode_event"
    is_thread_poll = source == "thread_poll_event"
    source_label = (
        "slack_socket_mode_grading"
        if is_socket_mode
        else ("slack_thread_poll_grading" if is_thread_poll else "slack_reply_fixture_grading")
    )
    submission_suffix = "SLACK-SOCKET" if is_socket_mode else ("SLACK-POLL" if is_thread_poll else "SLACK-REPLY-FIXTURE")
    return {
        "schema_version": "aicx.answer_submission.v0",
        "submission_id": f"SUB-{quiz_set['quiz_set_id']}-{submission_suffix}",
        "quiz_set_id": quiz_set["quiz_set_id"],
        "submitted_for_date": quiz_set["generated_for_date"],
        "source": source_label,
        "source_event_id": reply_event["reply_event_id"],
        "answers": answers,
    }


def grade_submission(quiz_set: dict[str, Any], submission: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    questions_by_id = {question["question_id"]: question for question in quiz_set["questions"]}
    answers_by_id = {answer["question_id"]: answer for answer in submission["answers"]}
    if set(questions_by_id) != set(answers_by_id):
        missing = sorted(set(questions_by_id) - set(answers_by_id))
        extra = sorted(set(answers_by_id) - set(questions_by_id))
        raise ValueError(f"submission questions do not match quiz set; missing={missing}, extra={extra}")

    graded_answers = []
    topic_totals: dict[str, dict[str, Any]] = {}
    correct_count = 0

    for question in quiz_set["questions"]:
        answer = answers_by_id[question["question_id"]]
        is_correct = answer["selected_choice"] == question["correct_choice"]
        correct_count += int(is_correct)
        topic = topic_totals.setdefault(
            question["topic_id"],
            {
                "topic_id": question["topic_id"],
                "label": question["topic_label"],
                "page_ranges": question["page_ranges"],
                "total": 0,
                "correct": 0,
            },
        )
        topic["total"] += 1
        topic["correct"] += int(is_correct)
        graded_answers.append(
            {
                "question_id": question["question_id"],
                "topic_id": question["topic_id"],
                "selected_choice": answer["selected_choice"],
                "correct_choice": question["correct_choice"],
                "is_correct": is_correct,
            }
        )

    total = len(quiz_set["questions"])
    topic_results = []
    for topic in sorted(topic_totals.values(), key=lambda item: item["topic_id"]):
        topic["accuracy"] = round(topic["correct"] / topic["total"], 4)
        topic_results.append(topic)

    grading_report = {
        "schema_version": "aicx.grading_report.v0",
        "grading_report_id": f"GRADING-{quiz_set['quiz_set_id']}",
        "quiz_set_id": quiz_set["quiz_set_id"],
        "submission_id": submission["submission_id"],
        "graded_for_date": quiz_set["generated_for_date"],
        "total_questions": total,
        "correct_count": correct_count,
        "accuracy": round(correct_count / total, 4),
        "topic_results": topic_results,
        "graded_answers": graded_answers,
    }
    recommendation = build_study_recommendation(quiz_set, grading_report)
    return grading_report, recommendation


def build_slack_grading_response(
    quiz_set: dict[str, Any],
    reply_event: dict[str, Any],
    submission: dict[str, Any],
    grading_report: dict[str, Any],
    recommendation: dict[str, Any],
) -> dict[str, Any]:
    topic_lines = [
        f"- {topic['label']}: {topic['correct']}/{topic['total']} ({topic['accuracy']:.0%})"
        for topic in grading_report["topic_results"]
    ]
    page_lines = [
        f"- {item['label']}: {json_page_range_label(item['page_ranges'])}"
        for item in recommendation["recommended_pages"]
    ]
    response_text = "\n".join(
        [
            f"採点結果: {grading_report['correct_count']}/{grading_report['total_questions']} ({grading_report['accuracy']:.0%})",
            "",
            "topic別正答率:",
            *topic_lines,
            "",
            "復習推奨ページ:",
            *page_lines,
        ]
    )

    return {
        "schema_version": "aicx.slack_grading_response.v0",
        "response_id": f"SLACKGRADING-{quiz_set['quiz_set_id']}",
        "quiz_set_id": quiz_set["quiz_set_id"],
        "reply_event_id": reply_event["reply_event_id"],
        "submission_id": submission["submission_id"],
        "grading_report_id": grading_report["grading_report_id"],
        "study_recommendation_id": recommendation["recommendation_id"],
        "generated_for_date": quiz_set["generated_for_date"],
        "delivery_channel": "slack",
        "thread_ts": reply_event["thread_ts"],
        "response_text": response_text,
        "total_questions": grading_report["total_questions"],
        "correct_count": grading_report["correct_count"],
        "accuracy": grading_report["accuracy"],
        "recommended_pages": recommendation["recommended_pages"],
        "slack_send_required": False,
    }


def write_slack_reply_grading_artifacts(
    quiz_set: dict[str, Any],
    reply_event: dict[str, Any],
    out_dir: Path,
) -> dict[str, Any]:
    submission = build_submission_from_slack_reply(quiz_set, reply_event)
    grading_report, recommendation = grade_submission(quiz_set, submission)
    response = build_slack_grading_response(quiz_set, reply_event, submission, grading_report, recommendation)
    write_json(out_dir / "slack_reply_event.json", reply_event)
    write_json(out_dir / "answer_submission.json", submission)
    write_json(out_dir / "grading_report.json", grading_report)
    write_json(out_dir / "study_recommendation.json", recommendation)
    write_json(out_dir / "slack_grading_response.json", response)
    return {
        "submission": submission,
        "grading_report": grading_report,
        "recommendation": recommendation,
        "slack_grading_response": response,
    }


def build_slack_reply_intake_receipt(
    quiz_set: dict[str, Any],
    mode: str,
    status: str,
    env_readiness: dict[str, Any],
    socket_mode_used: bool,
    slack_used: bool,
    started_at_unix_seconds: int,
    timeout_seconds: int | None = None,
    reply_event_id: str | None = None,
    grading_response_id: str | None = None,
    slack_response: dict[str, Any] | None = None,
    errors: list[str] | None = None,
) -> dict[str, Any]:
    return {
        "schema_version": "aicx.slack_reply_intake_receipt.v0",
        "intake_receipt_id": f"SLACKREPLYINTAKE-{quiz_set['quiz_set_id']}-{mode.upper()}",
        "quiz_set_id": quiz_set["quiz_set_id"],
        "generated_for_date": quiz_set["generated_for_date"],
        "delivery_channel": "slack",
        "mode": mode,
        "status": status,
        "socket_mode_used": socket_mode_used,
        "slack_used": slack_used,
        "target_channel_env": SLACK_CHANNEL_ENV,
        "target_app_token_env": SLACK_APP_TOKEN_ENV,
        "started_at_unix_seconds": started_at_unix_seconds,
        "completed_at_unix_seconds": int(time.time()),
        "timeout_seconds": timeout_seconds,
        "env_readiness": env_readiness,
        "received_reply_event_id": reply_event_id,
        "grading_response_id": grading_response_id,
        "slack_response": slack_response,
        "errors": errors or [],
    }


def build_slack_thread_poll_receipt(
    quiz_set: dict[str, Any],
    mode: str,
    status: str,
    env_readiness: dict[str, Any],
    channel_id: str | None,
    thread_ts: str | None,
    started_at_unix_seconds: int,
    thread_poll_used: bool,
    slack_used: bool,
    message_count: int = 0,
    candidate_reply_count: int = 0,
    reply_event_id: str | None = None,
    grading_response_id: str | None = None,
    slack_response: dict[str, Any] | None = None,
    errors: list[str] | None = None,
) -> dict[str, Any]:
    return {
        "schema_version": "aicx.slack_thread_poll_receipt.v0",
        "poll_receipt_id": f"SLACKTHREADPOLL-{quiz_set['quiz_set_id']}-{mode.upper()}",
        "quiz_set_id": quiz_set["quiz_set_id"],
        "generated_for_date": quiz_set["generated_for_date"],
        "delivery_channel": "slack",
        "mode": mode,
        "status": status,
        "thread_poll_used": thread_poll_used,
        "slack_used": slack_used,
        "target_channel_env": SLACK_CHANNEL_ENV,
        "started_at_unix_seconds": started_at_unix_seconds,
        "completed_at_unix_seconds": int(time.time()),
        "env_readiness": env_readiness,
        "channel_id": channel_id,
        "thread_ts": thread_ts,
        "message_count": message_count,
        "candidate_reply_count": candidate_reply_count,
        "received_reply_event_id": reply_event_id,
        "grading_response_id": grading_response_id,
        "slack_response": slack_response,
        "errors": errors or [],
    }


def write_slack_thread_poll_receipt_artifact(
    quiz_set: dict[str, Any],
    out_dir: Path,
    mode: str,
    status: str,
    env_readiness: dict[str, Any],
    channel_id: str | None,
    thread_ts: str | None,
    started_at_unix_seconds: int,
    thread_poll_used: bool,
    slack_used: bool,
    message_count: int = 0,
    candidate_reply_count: int = 0,
    reply_event_id: str | None = None,
    grading_response_id: str | None = None,
    slack_response: dict[str, Any] | None = None,
    errors: list[str] | None = None,
) -> dict[str, Any]:
    receipt = build_slack_thread_poll_receipt(
        quiz_set,
        mode,
        status,
        env_readiness,
        channel_id,
        thread_ts,
        started_at_unix_seconds,
        thread_poll_used,
        slack_used,
        message_count=message_count,
        candidate_reply_count=candidate_reply_count,
        reply_event_id=reply_event_id,
        grading_response_id=grading_response_id,
        slack_response=slack_response,
        errors=errors,
    )
    write_json(out_dir / "slack_thread_poll_receipt.json", receipt)
    return receipt


def build_maintenance_receipt(
    target_date: str,
    out_dir: Path,
    mode: str,
    status: str,
    action: str,
    run_state_path: Path,
    started_at_unix_seconds: int,
    allow_late: bool,
    late_window_hours: int,
    env_readiness: dict[str, Any],
    state: dict[str, Any] | None = None,
    actions: list[dict[str, Any]] | None = None,
    errors: list[str] | None = None,
) -> dict[str, Any]:
    artifacts = dict(state.get("artifacts", {}) if state else daily_run_artifact_paths(out_dir))
    artifacts["maintenance_receipt"] = str(out_dir / "maintenance_receipt.json")
    last_event = state.get("last_event", {}) if state else {}
    return {
        "schema_version": MAINTENANCE_RECEIPT_SCHEMA_VERSION,
        "maintenance_receipt_id": f"MAINT-AICX-{target_date}-{mode.upper()}",
        "date": target_date,
        "mode": mode,
        "status": status,
        "action": action,
        "actions": actions or [],
        "out_dir": str(out_dir),
        "run_state_path": str(run_state_path),
        "allow_late": allow_late,
        "late_window_hours": late_window_hours,
        "started_at_unix_seconds": started_at_unix_seconds,
        "completed_at_unix_seconds": int(time.time()),
        "env_readiness": env_readiness,
        "run_state_status": state.get("status") if state else None,
        "run_state_last_event_status": last_event.get("status"),
        "artifacts": artifacts,
        "errors": errors or [],
    }


def write_maintenance_receipt_artifact(
    target_date: str,
    out_dir: Path,
    mode: str,
    status: str,
    action: str,
    run_state_path: Path,
    started_at_unix_seconds: int,
    allow_late: bool,
    late_window_hours: int,
    actions: list[dict[str, Any]] | None = None,
    errors: list[str] | None = None,
) -> dict[str, Any]:
    out_dir.mkdir(parents=True, exist_ok=True)
    state = load_json_if_exists(run_state_path)
    if state is not None:
        state.setdefault("artifacts", {})["maintenance_receipt"] = str(out_dir / "maintenance_receipt.json")
        write_json(run_state_path, state)
    receipt = build_maintenance_receipt(
        target_date,
        out_dir,
        mode,
        status,
        action,
        run_state_path,
        started_at_unix_seconds,
        allow_late,
        late_window_hours,
        slack_env_readiness(),
        state=state,
        actions=actions,
        errors=errors,
    )
    write_json(out_dir / "maintenance_receipt.json", receipt)
    return receipt


def build_invalid_reply_error_text(error: Exception | str, total_questions: int | None = None) -> str:
    expected_count = total_questions or "全"
    return "\n".join(
        [
            "回答を採点できませんでした。",
            f"回答形式は `1:B 2:C ...` のように、問題番号とA-Dの選択肢を{expected_count}問分送ってください。",
            f"理由: {error}",
        ]
    )


def build_study_recommendation(quiz_set: dict[str, Any], grading_report: dict[str, Any]) -> dict[str, Any]:
    weak_topics = [
        topic for topic in grading_report["topic_results"] if topic["accuracy"] < 0.8
    ]
    if not weak_topics:
        weak_topics = sorted(
            grading_report["topic_results"],
            key=lambda item: (item["accuracy"], item["topic_id"]),
        )[:2]

    recommended_pages = []
    for topic in weak_topics:
        recommended_pages.append(
            {
                "topic_id": topic["topic_id"],
                "label": topic["label"],
                "page_ranges": topic["page_ranges"],
                "reason": f"topic accuracy {topic['accuracy']:.0%} is below the 80% fixture threshold",
            }
        )

    return {
        "schema_version": "aicx.study_recommendation.v0",
        "recommendation_id": f"RECO-{quiz_set['quiz_set_id']}",
        "quiz_set_id": quiz_set["quiz_set_id"],
        "grading_report_id": grading_report["grading_report_id"],
        "generated_for_date": quiz_set["generated_for_date"],
        "overall_accuracy": grading_report["accuracy"],
        "recommended_pages": recommended_pages,
        "next_actions": [
            "弱点 topic の page_ranges を優先して読む",
            "PDF ingest 前でも topic_map の手動 fixture を使って再出題する",
            "80% 未満の topic は翌日の fixture quiz で再確認する",
        ],
        "pdf_ingest_required": False,
        "slack_required": False,
    }


def build_slack_outbound_message(
    quiz_set: dict[str, Any],
    artifact_path: str,
    preview_count: int = 3,
    dispatch_notice: str | None = None,
) -> dict[str, Any]:
    included_questions = quiz_set["questions"]
    study_ranges = [parse_page_range(item) for item in quiz_set["study_window"]["study_pages"]]
    bank_count = sum(1 for question in quiz_set["questions"] if question["source"] == "question_bank_fixture")
    fallback_count = len(quiz_set["questions"]) - bank_count
    question_lines = []
    for index, question in enumerate(included_questions, start=1):
        choice_lines = [
            f"{choice['choice']}. {choice['text']}"
            for choice in question["choices"]
        ]
        question_lines.append(
            "\n".join(
                [
                    f"Q{index}. {question['prompt']}",
                    *choice_lines,
                ]
            )
        )

    message_sections = ["おはようございます。今日のAICX朝トレです。"]
    if dispatch_notice:
        message_sections.append(dispatch_notice)
    message_sections.extend(
        [
            "\n".join(
                [
                    f"範囲: {page_range_label(study_ranges)}",
                    f"今日の問題: {quiz_set['question_count']}問",
                    f"Bank品質問題: {bank_count}問",
                    f"Fallback問題: {fallback_count}問",
                    "回答形式: 1:B 2:C ...",
                ]
            ),
            "問題:",
            *question_lines,
            f"全文: {artifact_path}",
        ]
    )
    message_text = "\n\n".join(message_sections)

    return {
        "schema_version": "aicx.slack_outbound_message.v0",
        "message_id": f"SLACKMSG-{quiz_set['quiz_set_id']}",
        "quiz_set_id": quiz_set["quiz_set_id"],
        "generated_for_date": quiz_set["generated_for_date"],
        "delivery_channel": "slack",
        "target_channel_env": SLACK_CHANNEL_ENV,
        "artifact_path": artifact_path,
        "preview_count": len(included_questions),
        "total_questions": quiz_set["question_count"],
        "bank_question_count": bank_count,
        "fallback_question_count": fallback_count,
        "message_text": message_text,
        "dispatch_notice": dispatch_notice,
        "preview_questions": [
            {
                "question_number": index,
                "question_id": question["question_id"],
                "topic_id": question["topic_id"],
                "topic_label": question["topic_label"],
                "page_ranges": question["page_ranges"],
                "prompt": question["prompt"],
                "choices": question["choices"],
            }
            for index, question in enumerate(included_questions, start=1)
        ],
        "required_env": [SLACK_TOKEN_ENV, SLACK_CHANNEL_ENV],
        "live_send_supported": True,
    }


def slack_env_readiness(env: dict[str, str] | None = None, require_app_token: bool = False) -> dict[str, Any]:
    values = os.environ if env is None else env
    required_env = [SLACK_TOKEN_ENV, SLACK_CHANNEL_ENV]
    if require_app_token:
        required_env.append(SLACK_APP_TOKEN_ENV)
    missing_env = [
        name
        for name in required_env
        if not values.get(name, "").strip()
    ]
    readiness = {
        "bot_token_present": SLACK_TOKEN_ENV not in missing_env,
        "channel_id_present": SLACK_CHANNEL_ENV not in missing_env,
        "slack_sdk_available": module_available("slack_sdk"),
        "required_env": required_env,
        "missing_env": missing_env,
    }
    if require_app_token:
        readiness["app_token_present"] = SLACK_APP_TOKEN_ENV not in missing_env
        readiness["socket_mode_sdk_available"] = module_available("slack_sdk.socket_mode")
    return readiness


def build_slack_delivery_receipt(
    message: dict[str, Any],
    mode: str,
    env_readiness: dict[str, Any],
    status: str,
    slack_used: bool,
    slack_response: dict[str, Any] | None = None,
    errors: list[str] | None = None,
    dispatch_timing: dict[str, Any] | None = None,
) -> dict[str, Any]:
    return {
        "schema_version": "aicx.slack_delivery_receipt.v0",
        "delivery_receipt_id": f"SLACKRECEIPT-{message['quiz_set_id']}-{mode.upper()}",
        "message_id": message["message_id"],
        "quiz_set_id": message["quiz_set_id"],
        "generated_for_date": message["generated_for_date"],
        "delivery_channel": "slack",
        "mode": mode,
        "status": status,
        "slack_used": slack_used,
        "target_channel_env": SLACK_CHANNEL_ENV,
        "posted_at_unix_seconds": int(time.time()) if slack_used else None,
        "env_readiness": env_readiness,
        "slack_response": slack_response,
        "dispatch_timing_status": dispatch_timing.get("status") if dispatch_timing else None,
        "scheduled_at": dispatch_timing.get("scheduled_at") if dispatch_timing else None,
        "late_by_seconds": dispatch_timing.get("late_by_seconds") if dispatch_timing else None,
        "late_window_hours": dispatch_timing.get("late_window_hours") if dispatch_timing else None,
        "errors": errors or [],
    }


def post_slack_message_live(message: dict[str, Any], token: str, channel_id: str) -> dict[str, Any]:
    try:
        from slack_sdk import WebClient
    except ModuleNotFoundError as error:
        raise RuntimeError("slack_sdk is not installed; install it locally before live Slack smoke") from error

    client = WebClient(token=token)
    response = client.chat_postMessage(channel=channel_id, text=message["message_text"])
    data = getattr(response, "data", None)
    if isinstance(data, dict):
        return {
            "ok": bool(data.get("ok")),
            "channel": data.get("channel"),
            "ts": data.get("ts"),
        }
    return {"ok": bool(response.get("ok"))}


def write_slack_delivery_artifacts(
    quiz_set: dict[str, Any],
    out_dir: Path,
    artifact_path: str,
    preview_count: int,
    mode: str,
    raise_on_failure: bool = True,
    dispatch_timing: dict[str, Any] | None = None,
) -> tuple[dict[str, Any], dict[str, Any]]:
    message = build_slack_outbound_message(
        quiz_set,
        artifact_path,
        preview_count,
        dispatch_notice=render_dispatch_notice(dispatch_timing),
    )
    env_readiness = slack_env_readiness()

    if mode == "dry-run":
        receipt = build_slack_delivery_receipt(
            message,
            mode,
            env_readiness,
            "dry_run_ready",
            False,
            dispatch_timing=dispatch_timing,
        )
        write_json(out_dir / "slack_outbound_message.json", message)
        write_json(out_dir / "slack_delivery_receipt.json", receipt)
        return message, receipt

    errors = [f"missing {name}" for name in env_readiness["missing_env"]]
    if not env_readiness["slack_sdk_available"]:
        errors.append("slack_sdk is not installed")
    if errors:
        receipt = build_slack_delivery_receipt(
            message,
            mode,
            env_readiness,
            "blocked_missing_env_or_sdk",
            False,
            errors=errors,
            dispatch_timing=dispatch_timing,
        )
        write_json(out_dir / "slack_outbound_message.json", message)
        write_json(out_dir / "slack_delivery_receipt.json", receipt)
        if raise_on_failure:
            raise RuntimeError("; ".join(errors))
        return message, receipt

    try:
        response = post_slack_message_live(
            message,
            os.environ[SLACK_TOKEN_ENV],
            os.environ[SLACK_CHANNEL_ENV],
        )
        receipt = build_slack_delivery_receipt(
            message,
            mode,
            env_readiness,
            "sent",
            True,
            slack_response=response,
            dispatch_timing=dispatch_timing,
        )
    except Exception as error:
        receipt = build_slack_delivery_receipt(
            message,
            mode,
            env_readiness,
            "send_failed",
            True,
            errors=[str(error)],
            dispatch_timing=dispatch_timing,
        )

    write_json(out_dir / "slack_outbound_message.json", message)
    write_json(out_dir / "slack_delivery_receipt.json", receipt)
    if receipt["status"] != "sent" and raise_on_failure:
        raise RuntimeError("; ".join(receipt["errors"]) or receipt["status"])
    return message, receipt


def post_slack_thread_message_live(text: str, token: str, channel_id: str, thread_ts: str) -> dict[str, Any]:
    try:
        from slack_sdk import WebClient
    except ModuleNotFoundError as error:
        raise RuntimeError("slack_sdk is not installed; install it locally before live Slack smoke") from error

    client = WebClient(token=token)
    response = client.chat_postMessage(channel=channel_id, text=text, thread_ts=thread_ts)
    data = getattr(response, "data", None)
    if isinstance(data, dict):
        return {
            "ok": bool(data.get("ok")),
            "channel": data.get("channel"),
            "ts": data.get("ts"),
            "thread_ts": (
                data.get("message", {}).get("thread_ts") or thread_ts
                if isinstance(data.get("message"), dict)
                else thread_ts
            ),
        }
    return {"ok": bool(response.get("ok"))}


def build_slack_grading_delivery_receipt(
    quiz_set: dict[str, Any],
    mode: str,
    status: str,
    env_readiness: dict[str, Any],
    response_kind: str,
    thread_ts: str,
    message_text: str,
    slack_used: bool,
    reply_event_id: str | None = None,
    grading_response_id: str | None = None,
    slack_response: dict[str, Any] | None = None,
    errors: list[str] | None = None,
) -> dict[str, Any]:
    return {
        "schema_version": "aicx.slack_grading_delivery_receipt.v0",
        "delivery_receipt_id": f"SLACKGRADINGRECEIPT-{quiz_set['quiz_set_id']}-{mode.upper()}",
        "quiz_set_id": quiz_set["quiz_set_id"],
        "reply_event_id": reply_event_id,
        "grading_response_id": grading_response_id,
        "generated_for_date": quiz_set["generated_for_date"],
        "delivery_channel": "slack",
        "mode": mode,
        "status": status,
        "response_kind": response_kind,
        "slack_used": slack_used,
        "target_channel_env": SLACK_CHANNEL_ENV,
        "thread_ts": thread_ts,
        "message_text": message_text,
        "posted_at_unix_seconds": int(time.time()) if slack_used else None,
        "env_readiness": env_readiness,
        "slack_response": slack_response,
        "errors": errors or [],
    }


def write_slack_grading_delivery_receipt(
    quiz_set: dict[str, Any],
    out_dir: Path,
    mode: str,
    response_kind: str,
    thread_ts: str,
    message_text: str,
    reply_event_id: str | None = None,
    grading_response_id: str | None = None,
    raise_on_failure: bool = True,
) -> dict[str, Any]:
    env_readiness = slack_env_readiness()
    if mode == "dry-run":
        receipt = build_slack_grading_delivery_receipt(
            quiz_set,
            mode,
            "dry_run_ready",
            env_readiness,
            response_kind,
            thread_ts,
            message_text,
            False,
            reply_event_id=reply_event_id,
            grading_response_id=grading_response_id,
        )
        write_json(out_dir / "slack_grading_delivery_receipt.json", receipt)
        return receipt

    errors = [f"missing {name}" for name in env_readiness["missing_env"]]
    if not env_readiness["slack_sdk_available"]:
        errors.append("slack_sdk is not installed")
    if errors:
        receipt = build_slack_grading_delivery_receipt(
            quiz_set,
            mode,
            "blocked_missing_env_or_sdk",
            env_readiness,
            response_kind,
            thread_ts,
            message_text,
            False,
            reply_event_id=reply_event_id,
            grading_response_id=grading_response_id,
            errors=errors,
        )
        write_json(out_dir / "slack_grading_delivery_receipt.json", receipt)
        if raise_on_failure:
            raise RuntimeError("; ".join(errors))
        return receipt

    try:
        slack_response = post_slack_thread_message_live(
            message_text,
            os.environ[SLACK_TOKEN_ENV],
            os.environ[SLACK_CHANNEL_ENV],
            thread_ts,
        )
        receipt = build_slack_grading_delivery_receipt(
            quiz_set,
            mode,
            "sent",
            env_readiness,
            response_kind,
            thread_ts,
            message_text,
            True,
            reply_event_id=reply_event_id,
            grading_response_id=grading_response_id,
            slack_response=slack_response,
        )
    except Exception as error:
        receipt = build_slack_grading_delivery_receipt(
            quiz_set,
            mode,
            "send_failed",
            env_readiness,
            response_kind,
            thread_ts,
            message_text,
            True,
            reply_event_id=reply_event_id,
            grading_response_id=grading_response_id,
            errors=[str(error)],
        )
    write_json(out_dir / "slack_grading_delivery_receipt.json", receipt)
    if receipt["status"] != "sent" and raise_on_failure:
        raise RuntimeError("; ".join(receipt["errors"]) or receipt["status"])
    return receipt


def daily_run_artifact_paths(out_dir: Path) -> dict[str, str]:
    return {
        "quiz_set": str(out_dir / "quiz_set.json"),
        "quiz_prompt_json": str(out_dir / "quiz_prompt.json"),
        "quiz_prompt_md": str(out_dir / "quiz_prompt.md"),
        "slack_outbound_message": str(out_dir / "slack_outbound_message.json"),
        "slack_delivery_receipt": str(out_dir / "slack_delivery_receipt.json"),
        "slack_reply_event": str(out_dir / "slack_reply_event.json"),
        "answer_submission": str(out_dir / "answer_submission.json"),
        "grading_report": str(out_dir / "grading_report.json"),
        "study_recommendation": str(out_dir / "study_recommendation.json"),
        "slack_grading_response": str(out_dir / "slack_grading_response.json"),
        "slack_grading_delivery_receipt": str(out_dir / "slack_grading_delivery_receipt.json"),
        "slack_reply_intake_receipt": str(out_dir / "slack_reply_intake_receipt.json"),
        "slack_thread_poll_receipt": str(out_dir / "slack_thread_poll_receipt.json"),
        "maintenance_receipt": str(out_dir / "maintenance_receipt.json"),
        "run_state": str(out_dir / "run_state.json"),
    }


def build_daily_run_state(
    quiz_set: dict[str, Any],
    out_dir: Path,
    now_value: str | None,
    status: str,
) -> dict[str, Any]:
    timezone_name = quiz_set.get("timezone", "Asia/Tokyo")
    delivery_time = quiz_set.get("delivery_time", "04:30")
    now = parse_runner_now(now_value, timezone_name)
    scheduled_at = scheduled_datetime(quiz_set["generated_for_date"], delivery_time, timezone_name)
    return {
        "schema_version": DAILY_RUN_STATE_SCHEMA_VERSION,
        "run_id": f"AICX-DAILY-{quiz_set['generated_for_date']}",
        "quiz_set_id": quiz_set["quiz_set_id"],
        "date": quiz_set["generated_for_date"],
        "timezone": timezone_name,
        "scheduled_time": delivery_time,
        "scheduled_at": scheduled_at.isoformat(),
        "status": status,
        "steps": {
            "quiz_generated": False,
            "slack_sent": False,
            "reply_received": False,
            "graded": False,
            "grading_response_sent": False,
        },
        "slack": {
            "channel_id": None,
            "message_ts": None,
            "thread_ts": None,
            "slack_used": False,
        },
        "artifacts": daily_run_artifact_paths(out_dir),
        "idempotency": {
            "dispatch_key": f"{quiz_set['generated_for_date']}:{quiz_set['quiz_set_id']}:{SLACK_CHANNEL_ENV}",
            "grading_key": f"{quiz_set['generated_for_date']}:{quiz_set['quiz_set_id']}:thread",
            "duplicate_dispatch_skipped": False,
            "duplicate_grading_skipped": False,
            "processed_reply_event_ids": [],
        },
        "failure": None,
        "last_event": {
            "event_type": "initialized",
            "status": status,
            "at_unix_seconds": int(now.timestamp()),
        },
        "created_at_unix_seconds": int(now.timestamp()),
        "updated_at_unix_seconds": int(now.timestamp()),
    }


def mark_daily_run_event(
    state: dict[str, Any],
    event_type: str,
    status: str,
    now_value: str | None,
    errors: list[str] | None = None,
) -> None:
    now = parse_runner_now(now_value, state.get("timezone", "Asia/Tokyo"))
    state["updated_at_unix_seconds"] = int(now.timestamp())
    state["last_event"] = {
        "event_type": event_type,
        "status": status,
        "at_unix_seconds": int(now.timestamp()),
    }
    if errors:
        state["failure"] = {
            "event_type": event_type,
            "errors": errors,
            "at_unix_seconds": int(now.timestamp()),
        }


def load_or_new_daily_run_state(
    state_path: Path,
    quiz_set: dict[str, Any],
    out_dir: Path,
    now_value: str | None,
    status: str,
) -> dict[str, Any]:
    if state_path.exists():
        return load_json(state_path)
    return build_daily_run_state(quiz_set, out_dir, now_value, status)


def update_state_after_dispatch(
    state: dict[str, Any],
    receipt: dict[str, Any],
    now_value: str | None,
    event_status: str | None = None,
) -> None:
    success_statuses = {"dry_run_ready", "sent"}
    state["steps"]["quiz_generated"] = True
    if receipt["status"] in success_statuses:
        state["steps"]["slack_sent"] = True
        state["status"] = "dispatched"
        state["failure"] = None
    else:
        state["status"] = "failed"
    slack_response = receipt.get("slack_response") or {}
    state["slack"] = {
        "channel_id": slack_response.get("channel"),
        "message_ts": slack_response.get("ts"),
        "thread_ts": slack_response.get("ts"),
        "slack_used": bool(receipt.get("slack_used")),
    }
    mark_daily_run_event(
        state,
        "dispatch",
        event_status or receipt["status"],
        now_value,
        receipt.get("errors") or None,
    )


def update_state_after_grading(
    state: dict[str, Any],
    reply_event: dict[str, Any],
    delivery_receipt: dict[str, Any],
    graded: bool,
    now_value: str | None,
) -> None:
    success_statuses = {"dry_run_ready", "sent"}
    response_sent = delivery_receipt["status"] in success_statuses
    state["steps"]["reply_received"] = True
    state["steps"]["graded"] = bool(graded)
    state["steps"]["grading_response_sent"] = response_sent
    state["status"] = "graded" if graded and response_sent else ("invalid_reply" if response_sent else "failed")
    state["idempotency"].setdefault("processed_reply_event_ids", [])
    if reply_event["reply_event_id"] not in state["idempotency"]["processed_reply_event_ids"]:
        state["idempotency"]["processed_reply_event_ids"].append(reply_event["reply_event_id"])
    slack_response = delivery_receipt.get("slack_response") or {}
    if slack_response.get("channel"):
        state["slack"]["channel_id"] = slack_response.get("channel")
    if delivery_receipt.get("thread_ts"):
        state["slack"]["thread_ts"] = delivery_receipt["thread_ts"]
    mark_daily_run_event(
        state,
        "grading",
        delivery_receipt["status"],
        now_value,
        delivery_receipt.get("errors") or None,
    )


def delivery_receipt_succeeded(delivery_receipt: dict[str, Any]) -> bool:
    return delivery_receipt["status"] in {"dry_run_ready", "sent"}


def resolve_run_state_reply_target(
    state: dict[str, Any],
    expected_channel_id: str | None = None,
    expected_thread_ts: str | None = None,
) -> tuple[str | None, str | None]:
    slack_state = state.get("slack", {})
    return (
        expected_channel_id or slack_state.get("channel_id") or os.environ.get(SLACK_CHANNEL_ENV),
        expected_thread_ts or slack_state.get("thread_ts"),
    )


def write_socket_reply_intake_receipt_artifact(
    quiz_set: dict[str, Any],
    out_dir: Path,
    mode: str,
    status: str,
    env_readiness: dict[str, Any],
    socket_mode_used: bool,
    slack_used: bool,
    started_at_unix_seconds: int,
    timeout_seconds: int | None = None,
    reply_event_id: str | None = None,
    grading_response_id: str | None = None,
    slack_response: dict[str, Any] | None = None,
    errors: list[str] | None = None,
) -> dict[str, Any]:
    receipt = build_slack_reply_intake_receipt(
        quiz_set,
        mode,
        status,
        env_readiness,
        socket_mode_used,
        slack_used,
        started_at_unix_seconds,
        timeout_seconds=timeout_seconds,
        reply_event_id=reply_event_id,
        grading_response_id=grading_response_id,
        slack_response=slack_response,
        errors=errors or [],
    )
    write_json(out_dir / "slack_reply_intake_receipt.json", receipt)
    return receipt


def write_run_state_socket_reply_failure(
    quiz_set: dict[str, Any],
    out_dir: Path,
    run_state_path: Path,
    mode: str,
    status: str,
    env_readiness: dict[str, Any],
    socket_mode_used: bool,
    started_at_unix_seconds: int,
    now_value: str | None,
    timeout_seconds: int | None,
    errors: list[str],
) -> dict[str, Any]:
    state = load_json(run_state_path)
    state["status"] = "failed"
    mark_daily_run_event(state, "socket_reply", status, now_value, errors)
    write_json(run_state_path, state)
    return write_socket_reply_intake_receipt_artifact(
        quiz_set,
        out_dir,
        mode,
        status,
        env_readiness,
        socket_mode_used,
        False,
        started_at_unix_seconds,
        timeout_seconds=timeout_seconds,
        errors=errors,
    )


def process_slack_reply_event_with_run_state(
    quiz_set: dict[str, Any],
    reply_event: dict[str, Any],
    out_dir: Path,
    run_state_path: Path,
    mode: str,
    env_readiness: dict[str, Any],
    socket_mode_used: bool,
    started_at_unix_seconds: int,
    now_value: str | None,
    timeout_seconds: int | None = None,
) -> dict[str, Any]:
    state = load_json(run_state_path)
    processed = state["idempotency"].setdefault("processed_reply_event_ids", [])

    expected_thread_ts = state.get("slack", {}).get("thread_ts")
    if expected_thread_ts and reply_event.get("thread_ts") != expected_thread_ts:
        error = f"reply thread_ts {reply_event.get('thread_ts')} does not match run_state thread_ts {expected_thread_ts}"
        state["status"] = "failed"
        mark_daily_run_event(state, "socket_reply", "thread_mismatch", now_value, [error])
        write_json(run_state_path, state)
        return write_socket_reply_intake_receipt_artifact(
            quiz_set,
            out_dir,
            mode,
            "thread_mismatch",
            env_readiness,
            socket_mode_used,
            False,
            started_at_unix_seconds,
            timeout_seconds=timeout_seconds,
            reply_event_id=reply_event["reply_event_id"],
            errors=[error],
        )

    if state["steps"].get("graded") or reply_event["reply_event_id"] in processed:
        state["idempotency"]["duplicate_grading_skipped"] = True
        mark_daily_run_event(state, "socket_reply", SOCKET_REPLY_LISTEN_DUPLICATE_STATUS, now_value)
        write_json(run_state_path, state)
        return write_socket_reply_intake_receipt_artifact(
            quiz_set,
            out_dir,
            mode,
            SOCKET_REPLY_LISTEN_DUPLICATE_STATUS,
            env_readiness,
            socket_mode_used,
            False,
            started_at_unix_seconds,
            timeout_seconds=timeout_seconds,
            reply_event_id=reply_event["reply_event_id"],
        )

    write_json(out_dir / "slack_reply_event.json", reply_event)
    try:
        artifacts = write_slack_reply_grading_artifacts(quiz_set, reply_event, out_dir)
        delivery_receipt = write_slack_grading_delivery_receipt(
            quiz_set,
            out_dir,
            mode,
            "grading_result",
            reply_event["thread_ts"],
            artifacts["slack_grading_response"]["response_text"],
            reply_event_id=reply_event["reply_event_id"],
            grading_response_id=artifacts["slack_grading_response"]["response_id"],
            raise_on_failure=False,
        )
        update_state_after_grading(state, reply_event, delivery_receipt, True, now_value)
        status = (
            SOCKET_REPLY_LISTEN_SENT_STATUS
            if delivery_receipt_succeeded(delivery_receipt)
            else "grading_response_send_failed"
        )
        errors = delivery_receipt.get("errors", [])
        grading_response_id = artifacts["slack_grading_response"]["response_id"]
    except Exception as error:
        delivery_receipt = write_slack_grading_delivery_receipt(
            quiz_set,
            out_dir,
            mode,
            "invalid_reply_error",
            reply_event["thread_ts"],
            build_invalid_reply_error_text(error, quiz_set["question_count"]),
            reply_event_id=reply_event["reply_event_id"],
            raise_on_failure=False,
        )
        update_state_after_grading(state, reply_event, delivery_receipt, False, now_value)
        state["failure"] = {
            "event_type": "socket_reply",
            "errors": [str(error), *delivery_receipt.get("errors", [])],
            "at_unix_seconds": state["updated_at_unix_seconds"],
        }
        status = (
            SOCKET_REPLY_LISTEN_INVALID_SENT_STATUS
            if delivery_receipt_succeeded(delivery_receipt)
            else "invalid_reply_error_send_failed"
        )
        errors = [str(error), *delivery_receipt.get("errors", [])]
        grading_response_id = None

    mark_daily_run_event(state, "socket_reply", status, now_value, errors or None)
    write_json(run_state_path, state)
    return write_socket_reply_intake_receipt_artifact(
        quiz_set,
        out_dir,
        mode,
        status,
        env_readiness,
        socket_mode_used,
        bool(delivery_receipt.get("slack_used")),
        started_at_unix_seconds,
        timeout_seconds=timeout_seconds,
        reply_event_id=reply_event["reply_event_id"],
        grading_response_id=grading_response_id,
        slack_response=delivery_receipt.get("slack_response"),
        errors=errors,
    )


def resolve_run_state_artifact_path(
    run_state_path: Path,
    state: dict[str, Any],
    artifact_key: str,
    explicit_path: Path | None = None,
) -> Path:
    if explicit_path is not None:
        return explicit_path

    artifact_value = state.get("artifacts", {}).get(artifact_key)
    if not artifact_value:
        raise ValueError(f"run_state.artifacts.{artifact_key} is required")

    artifact_path = Path(artifact_value)
    if artifact_path.is_absolute() or artifact_path.exists():
        return artifact_path

    local_name_path = run_state_path.parent / artifact_path.name
    if local_name_path.exists():
        return local_name_path

    return artifact_path


def fetch_slack_thread_messages_live(token: str, channel_id: str, thread_ts: str) -> list[dict[str, Any]]:
    try:
        from slack_sdk import WebClient
    except ModuleNotFoundError as error:
        raise RuntimeError("slack_sdk is not installed; install it locally before Slack thread polling") from error

    client = WebClient(token=token)
    cursor = None
    messages: list[dict[str, Any]] = []
    while True:
        kwargs: dict[str, Any] = {
            "channel": channel_id,
            "ts": thread_ts,
            "limit": 200,
        }
        if cursor:
            kwargs["cursor"] = cursor
        response = client.conversations_replies(**kwargs)
        data = getattr(response, "data", response)
        if not isinstance(data, dict):
            raise RuntimeError("Slack conversations.replies returned an unexpected response")
        messages.extend(data.get("messages", []))
        cursor = (data.get("response_metadata") or {}).get("next_cursor")
        if not cursor:
            return messages


def slack_api_error_code(error: Exception) -> str | None:
    response = getattr(error, "response", None)
    if isinstance(response, dict):
        value = response.get("error")
        return str(value) if value else None
    try:
        value = response["error"]  # type: ignore[index]
    except Exception:
        return None
    return str(value) if value else None


def map_thread_poll_failure_status(error: Exception) -> str:
    error_code = slack_api_error_code(error)
    if error_code == "ratelimited":
        return "rate_limited"
    if error_code in {"channel_not_found", "message_not_found", "thread_not_found", "not_in_channel"}:
        return "thread_not_found"
    return "poll_failed"


def map_thread_poll_processing_status(intake_status: str) -> str:
    return {
        SOCKET_REPLY_LISTEN_SENT_STATUS: SLACK_THREAD_POLL_GRADED_STATUS,
        SOCKET_REPLY_LISTEN_SUCCESS_STATUS: SLACK_THREAD_POLL_GRADED_STATUS,
        SOCKET_REPLY_LISTEN_INVALID_SENT_STATUS: SLACK_THREAD_POLL_INVALID_STATUS,
        SOCKET_REPLY_LISTEN_DUPLICATE_STATUS: SLACK_THREAD_POLL_DUPLICATE_STATUS,
    }.get(intake_status, "poll_failed")


def command_daily_dispatch(args: argparse.Namespace) -> None:
    load_env_file(args.env_file)
    schedule = load_json(args.study_schedule)
    topic_map = load_json(args.topic_map)
    question_bank = load_json(args.question_bank) if args.question_bank else None
    adaptive_plan = load_json(args.adaptive_plan) if args.adaptive_plan else None
    if adaptive_plan:
        quiz_set = generate_quiz_set_from_adaptive_plan(
            schedule,
            topic_map,
            args.date,
            adaptive_plan,
            args.question_count,
            question_bank,
        )
    else:
        quiz_set = generate_quiz_set(schedule, topic_map, args.date, args.question_count, question_bank)
    state_path = args.run_state or args.out_dir / "run_state.json"
    state = load_or_new_daily_run_state(state_path, quiz_set, args.out_dir, args.now, "initialized")
    dispatch_timing = classify_daily_dispatch_timing(
        quiz_set["generated_for_date"],
        quiz_set["delivery_time"],
        quiz_set["timezone"],
        args.now,
        getattr(args, "allow_late", False),
        getattr(args, "late_window_hours", 12),
    )

    if state["steps"].get("slack_sent"):
        state["idempotency"]["duplicate_dispatch_skipped"] = True
        mark_daily_run_event(state, "dispatch", "duplicate_skipped", args.now)
        write_json(state_path, state)
        return

    if not args.force and dispatch_timing["status"] == "not_due":
        state["status"] = "not_due"
        mark_daily_run_event(state, "dispatch", "not_due", args.now)
        write_json(state_path, state)
        return

    if not args.force and dispatch_timing["status"] == "late_window_expired":
        state["status"] = "late_window_expired"
        mark_daily_run_event(state, "dispatch", "late_window_expired", args.now)
        write_json(state_path, state)
        return

    quiz_prompt = build_quiz_prompt(quiz_set, str(args.out_dir / "quiz_set.json"))
    write_json(args.out_dir / "quiz_set.json", quiz_set)
    if adaptive_plan:
        write_json(args.out_dir / "adaptive_plan.json", adaptive_plan)
    write_json(args.out_dir / "quiz_prompt.json", quiz_prompt)
    write_text(args.out_dir / "quiz_prompt.md", render_quiz_prompt_markdown(quiz_prompt))
    _, receipt = write_slack_delivery_artifacts(
        quiz_set,
        args.out_dir,
        str(args.out_dir / "quiz_prompt.md"),
        args.preview_count,
        args.mode,
        raise_on_failure=False,
        dispatch_timing=dispatch_timing if getattr(args, "allow_late", False) else None,
    )
    update_state_after_dispatch(
        state,
        receipt,
        args.now,
        event_status=(
            "late_dispatched"
            if dispatch_timing["status"] == "late_dispatched" and receipt["status"] in {"dry_run_ready", "sent"}
            else None
        ),
    )
    write_json(state_path, state)
    if receipt["status"] not in {"dry_run_ready", "sent"}:
        raise RuntimeError("; ".join(receipt["errors"]) or receipt["status"])


def command_daily_grade_poll(args: argparse.Namespace) -> None:
    load_env_file(args.env_file)
    started_at = int(time.time())
    state = load_json(args.run_state)
    out_dir = args.out_dir or args.run_state.parent
    quiz_set_path = resolve_run_state_artifact_path(args.run_state, state, "quiz_set", args.quiz_set)
    quiz_set = load_json(quiz_set_path)
    env_readiness = slack_env_readiness()
    channel_id, thread_ts = resolve_run_state_reply_target(state)

    if state["steps"].get("graded"):
        state["idempotency"]["duplicate_grading_skipped"] = True
        mark_daily_run_event(state, "thread_poll", SLACK_THREAD_POLL_DUPLICATE_STATUS, args.now)
        write_json(args.run_state, state)
        write_slack_thread_poll_receipt_artifact(
            quiz_set,
            out_dir,
            args.mode,
            SLACK_THREAD_POLL_DUPLICATE_STATUS,
            env_readiness,
            channel_id,
            thread_ts,
            started_at,
            False,
            False,
        )
        return

    if not channel_id or not thread_ts:
        errors = ["run_state slack.channel_id and slack.thread_ts are required before Slack thread polling"]
        state["status"] = "failed"
        mark_daily_run_event(state, "thread_poll", "thread_not_found", args.now, errors)
        write_json(args.run_state, state)
        write_slack_thread_poll_receipt_artifact(
            quiz_set,
            out_dir,
            args.mode,
            "thread_not_found",
            env_readiness,
            channel_id,
            thread_ts,
            started_at,
            False,
            False,
            errors=errors,
        )
        raise RuntimeError("; ".join(errors))

    thread_poll_used = False
    if args.thread_messages_fixture:
        thread_messages_fixture = load_json(args.thread_messages_fixture)
        messages = thread_messages_fixture.get("messages", [])
        if not isinstance(messages, list):
            raise ValueError("thread messages fixture must contain a messages array")
    elif args.mode == "dry-run":
        messages = []
    else:
        errors = [f"missing {name}" for name in env_readiness["missing_env"]]
        if not env_readiness["slack_sdk_available"]:
            errors.append("slack_sdk is not installed")
        if errors:
            state["status"] = "failed"
            mark_daily_run_event(state, "thread_poll", "blocked_missing_env_or_sdk", args.now, errors)
            write_json(args.run_state, state)
            write_slack_thread_poll_receipt_artifact(
                quiz_set,
                out_dir,
                args.mode,
                "blocked_missing_env_or_sdk",
                env_readiness,
                channel_id,
                thread_ts,
                started_at,
                False,
                False,
                errors=errors,
            )
            raise RuntimeError("; ".join(errors))
        try:
            messages = fetch_slack_thread_messages_live(os.environ[SLACK_TOKEN_ENV], channel_id, thread_ts)
            thread_poll_used = True
        except Exception as error:
            status = map_thread_poll_failure_status(error)
            errors = [str(error)]
            state["status"] = "failed"
            mark_daily_run_event(state, "thread_poll", status, args.now, errors)
            write_json(args.run_state, state)
            write_slack_thread_poll_receipt_artifact(
                quiz_set,
                out_dir,
                args.mode,
                status,
                env_readiness,
                channel_id,
                thread_ts,
                started_at,
                thread_poll_used,
                False,
                errors=errors,
            )
            raise RuntimeError("; ".join(errors))

    processed = state["idempotency"].setdefault("processed_reply_event_ids", [])
    reply_event, candidate_reply_count = select_unprocessed_thread_reply_event(
        quiz_set,
        messages,
        channel_id,
        thread_ts,
        processed,
    )

    if reply_event is None:
        status = SLACK_THREAD_POLL_DUPLICATE_STATUS if candidate_reply_count else SLACK_THREAD_POLL_NO_REPLY_STATUS
        if status == SLACK_THREAD_POLL_DUPLICATE_STATUS:
            state["idempotency"]["duplicate_grading_skipped"] = True
        if (
            status == SLACK_THREAD_POLL_NO_REPLY_STATUS
            and state.get("status") == "failed"
            and (state.get("failure") or {}).get("event_type") == "thread_poll"
        ):
            state["status"] = "dispatched"
            state["failure"] = None
        mark_daily_run_event(state, "thread_poll", status, args.now)
        write_json(args.run_state, state)
        write_slack_thread_poll_receipt_artifact(
            quiz_set,
            out_dir,
            args.mode,
            status,
            env_readiness,
            channel_id,
            thread_ts,
            started_at,
            thread_poll_used,
            False,
            message_count=len(messages),
            candidate_reply_count=candidate_reply_count,
        )
        return

    intake_receipt = process_slack_reply_event_with_run_state(
        quiz_set,
        reply_event,
        out_dir,
        args.run_state,
        args.mode,
        env_readiness,
        False,
        started_at,
        args.now,
    )
    poll_status = map_thread_poll_processing_status(intake_receipt["status"])
    poll_receipt = write_slack_thread_poll_receipt_artifact(
        quiz_set,
        out_dir,
        args.mode,
        poll_status,
        env_readiness,
        channel_id,
        thread_ts,
        started_at,
        thread_poll_used,
        bool(intake_receipt.get("slack_used")),
        message_count=len(messages),
        candidate_reply_count=candidate_reply_count,
        reply_event_id=intake_receipt.get("received_reply_event_id"),
        grading_response_id=intake_receipt.get("grading_response_id"),
        slack_response=intake_receipt.get("slack_response"),
        errors=intake_receipt.get("errors", []),
    )
    if poll_receipt["status"] in SLACK_THREAD_POLL_FAILURE_STATUSES:
        raise RuntimeError("; ".join(poll_receipt.get("errors", [])) or poll_receipt["status"])


def maintenance_status_after_dispatch(state: dict[str, Any] | None, receipt: dict[str, Any] | None) -> str:
    if receipt and receipt.get("status") not in {"dry_run_ready", "sent"}:
        receipt_status = receipt.get("status")
        return receipt_status if receipt_status == "blocked_missing_env_or_sdk" else "dispatch_failed"
    if not state:
        return "dispatch_failed"
    last_event_status = (state.get("last_event") or {}).get("status")
    if last_event_status == "late_dispatched":
        return "late_dispatched"
    if last_event_status == "duplicate_skipped":
        return "duplicate_dispatch_skipped"
    if state.get("status") in {"not_due", "late_window_expired"}:
        return state["status"]
    if state.get("steps", {}).get("slack_sent"):
        return "dispatched"
    if state.get("status") == "failed":
        return "dispatch_failed"
    return state.get("status", "dispatch_failed")


def command_daily_maintain(args: argparse.Namespace) -> None:
    load_env_file(args.env_file)
    started_at = int(time.time())
    out_dir = args.out_root / args.date
    run_state_path = out_dir / "run_state.json"
    out_dir.mkdir(parents=True, exist_ok=True)
    state = load_json_if_exists(run_state_path)

    if state and state.get("steps", {}).get("graded"):
        write_maintenance_receipt_artifact(
            args.date,
            out_dir,
            args.mode,
            "already_graded",
            "noop",
            run_state_path,
            started_at,
            args.allow_late,
            args.late_window_hours,
            actions=[{"action": "noop", "status": "already_graded"}],
        )
        return

    if state and state.get("steps", {}).get("slack_sent"):
        poll_args = argparse.Namespace(
            run_state=run_state_path,
            quiz_set=None,
            out_dir=out_dir,
            mode=args.mode,
            thread_messages_fixture=args.thread_messages_fixture,
            env_file=args.env_file,
            now=args.now,
        )
        try:
            command_daily_grade_poll(poll_args)
        except Exception as error:
            poll_receipt = load_json_if_exists(out_dir / "slack_thread_poll_receipt.json")
            if poll_receipt:
                status = poll_receipt.get("status", "poll_failed")
            else:
                fallback_state = load_json_if_exists(run_state_path) or {}
                last_event = fallback_state.get("last_event") or {}
                status = (
                    last_event.get("status")
                    if last_event.get("event_type") == "thread_poll"
                    else "poll_failed"
                ) or "poll_failed"
            write_maintenance_receipt_artifact(
                args.date,
                out_dir,
                args.mode,
                status,
                "thread_poll",
                run_state_path,
                started_at,
                args.allow_late,
                args.late_window_hours,
                actions=[{"action": "thread_poll", "status": status}],
                errors=(poll_receipt or {}).get("errors", [str(error)]),
            )
            raise
        poll_receipt = load_json(out_dir / "slack_thread_poll_receipt.json")
        write_maintenance_receipt_artifact(
            args.date,
            out_dir,
            args.mode,
            poll_receipt["status"],
            "thread_poll",
            run_state_path,
            started_at,
            args.allow_late,
            args.late_window_hours,
            actions=[{"action": "thread_poll", "status": poll_receipt["status"]}],
            errors=poll_receipt.get("errors", []),
        )
        return

    dispatch_args = argparse.Namespace(
        study_schedule=args.study_schedule,
        topic_map=args.topic_map,
        question_bank=args.question_bank,
        adaptive_plan=args.adaptive_plan,
        date=args.date,
        question_count=args.question_count,
        out_dir=out_dir,
        run_state=run_state_path,
        mode=args.mode,
        preview_count=args.preview_count,
        env_file=args.env_file,
        now=args.now,
        force=False,
        allow_late=args.allow_late,
        late_window_hours=args.late_window_hours,
    )
    try:
        command_daily_dispatch(dispatch_args)
    except Exception as error:
        state = load_json_if_exists(run_state_path)
        receipt = load_json_if_exists(out_dir / "slack_delivery_receipt.json")
        status = maintenance_status_after_dispatch(state, receipt)
        write_maintenance_receipt_artifact(
            args.date,
            out_dir,
            args.mode,
            status,
            "dispatch",
            run_state_path,
            started_at,
            args.allow_late,
            args.late_window_hours,
            actions=[{"action": "dispatch", "status": status}],
            errors=(receipt or {}).get("errors", [str(error)]),
        )
        raise

    state = load_json_if_exists(run_state_path)
    receipt = load_json_if_exists(out_dir / "slack_delivery_receipt.json")
    status = maintenance_status_after_dispatch(state, receipt)
    action = "noop" if status in {"not_due", "late_window_expired"} else "dispatch"
    write_maintenance_receipt_artifact(
        args.date,
        out_dir,
        args.mode,
        status,
        action,
        run_state_path,
        started_at,
        args.allow_late,
        args.late_window_hours,
        actions=[{"action": action, "status": status}],
        errors=(receipt or {}).get("errors", []),
    )


def format_daily_status_time(timestamp: int | None, timezone_name: str) -> str:
    if timestamp is None:
        return "unknown"
    return datetime.fromtimestamp(int(timestamp), ZoneInfo(timezone_name)).strftime("%Y-%m-%d %H:%M:%S %Z")


def derive_daily_status(state: dict[str, Any]) -> str:
    steps = state.get("steps", {})
    raw_status = state.get("status")
    slack_state = state.get("slack", {})
    if steps.get("graded"):
        return "graded"
    if raw_status in {"not_due", "late_window_expired"}:
        return raw_status
    if steps.get("slack_sent") and slack_state.get("thread_ts"):
        return "waiting_for_reply"
    if steps.get("slack_sent"):
        return "needs_recovery"
    if raw_status == "failed":
        return "failed"
    if steps.get("quiz_generated"):
        return "dispatch_pending"
    return raw_status or "unknown"


def build_daily_maintain_command(run_state_path: Path, target_date: str) -> str:
    out_root = run_state_path.parent.parent
    return (
        "python scripts/aicx_study_fixture.py daily-maintain "
        f"--date {target_date} "
        f"--out-root {out_root} "
        "--env-file .env.local --allow-late --late-window-hours 12"
    )


def daily_status_next_action(status: str, state: dict[str, Any], run_state_path: Path) -> str:
    target_date = state.get("date", "YYYY-MM-DD")
    maintain_command = build_daily_maintain_command(run_state_path, target_date)
    if status == "not_due":
        return f"04:30 JST以降に `{maintain_command}` を実行する。"
    if status == "waiting_for_reply":
        return f"Slack threadへ回答する。返信済みなら `{maintain_command}` を再実行する。"
    if status == "graded":
        return "今日分は完了。翌日のdaily-maintainを待つ。"
    if status == "late_window_expired":
        return "遅延送信windowを過ぎている。必要なら別runとして手動送信を判断する。"
    if status in {"failed", "needs_recovery"}:
        return f"`run_state.failure` と receipt を確認し、修正後に `{maintain_command}` を再実行する。"
    return f"状態を確認し、必要なら `{maintain_command}` を実行する。"


def build_daily_status_report(state: dict[str, Any], run_state_path: Path) -> dict[str, Any]:
    timezone_name = state.get("timezone", "Asia/Tokyo")
    status = derive_daily_status(state)
    artifacts = state.get("artifacts", {})
    artifact_keys = [
        "quiz_prompt_md",
        "run_state",
        "slack_delivery_receipt",
        "maintenance_receipt",
        "slack_thread_poll_receipt",
        "grading_report",
        "study_recommendation",
    ]
    return {
        "date": state.get("date"),
        "status": status,
        "raw_status": state.get("status"),
        "steps": state.get("steps", {}),
        "slack": state.get("slack", {}),
        "last_event": state.get("last_event", {}),
        "last_event_at": format_daily_status_time(
            (state.get("last_event") or {}).get("at_unix_seconds"),
            timezone_name,
        ),
        "failure": state.get("failure"),
        "next_action": daily_status_next_action(status, state, run_state_path),
        "artifacts": {
            key: artifacts[key]
            for key in artifact_keys
            if artifacts.get(key)
        },
    }


def render_daily_status_report(report: dict[str, Any]) -> str:
    slack_state = report["slack"]
    last_event = report["last_event"]
    steps = report["steps"]
    lines = [
        f"AICX Study Bot - {report['date']}",
        "",
        "Status:",
        f"  {report['status']}",
    ]
    if report["raw_status"] != report["status"]:
        lines.append(f"  raw_run_state: {report['raw_status']}")
    lines.extend(
        [
            "",
            "Steps:",
            f"  sent: {str(bool(steps.get('slack_sent'))).lower()}",
            f"  replied: {str(bool(steps.get('reply_received'))).lower()}",
            f"  graded: {str(bool(steps.get('graded'))).lower()}",
            "",
            "Slack:",
            f"  sent: {str(bool(steps.get('slack_sent'))).lower()}",
            f"  channel_id: {slack_state.get('channel_id') or '-'}",
            f"  thread_ts: {slack_state.get('thread_ts') or '-'}",
            "",
            "Last Event:",
            f"  {last_event.get('status') or '-'} at {report['last_event_at']}",
        ]
    )
    if report.get("failure"):
        lines.extend(
            [
                "",
                "Failure:",
                f"  event_type: {report['failure'].get('event_type')}",
                f"  errors: {'; '.join(report['failure'].get('errors', []))}",
            ]
        )
    lines.extend(["", "Next Action:", f"  {report['next_action']}", "", "Artifacts:"])
    for key, path in report["artifacts"].items():
        lines.append(f"  {key}: {path}")
    return "\n".join(lines) + "\n"


def command_daily_status(args: argparse.Namespace) -> None:
    state = load_json(args.run_state)
    report = build_daily_status_report(state, args.run_state)
    print(render_daily_status_report(report), end="")


def command_daily_grade(args: argparse.Namespace) -> None:
    load_env_file(args.env_file)
    state = load_json(args.run_state)
    quiz_set = load_json(args.quiz_set)
    reply_event = load_json(args.slack_reply_event)
    processed = state["idempotency"].setdefault("processed_reply_event_ids", [])

    if state["steps"].get("graded") or reply_event["reply_event_id"] in processed:
        state["idempotency"]["duplicate_grading_skipped"] = True
        mark_daily_run_event(state, "grading", "duplicate_skipped", args.now)
        write_json(args.run_state, state)
        return

    expected_thread_ts = state.get("slack", {}).get("thread_ts")
    if expected_thread_ts and reply_event.get("thread_ts") != expected_thread_ts:
        state["status"] = "failed"
        error = f"reply thread_ts {reply_event.get('thread_ts')} does not match run_state thread_ts {expected_thread_ts}"
        mark_daily_run_event(state, "grading", "thread_mismatch", args.now, [error])
        write_json(args.run_state, state)
        raise RuntimeError(error)

    try:
        artifacts = write_slack_reply_grading_artifacts(quiz_set, reply_event, args.out_dir)
        delivery_receipt = write_slack_grading_delivery_receipt(
            quiz_set,
            args.out_dir,
            args.mode,
            "grading_result",
            reply_event["thread_ts"],
            artifacts["slack_grading_response"]["response_text"],
            reply_event_id=reply_event["reply_event_id"],
            grading_response_id=artifacts["slack_grading_response"]["response_id"],
            raise_on_failure=False,
        )
        update_state_after_grading(state, reply_event, delivery_receipt, True, args.now)
    except Exception as error:
        delivery_receipt = write_slack_grading_delivery_receipt(
            quiz_set,
            args.out_dir,
            args.mode,
            "invalid_reply_error",
            reply_event["thread_ts"],
            build_invalid_reply_error_text(error, quiz_set["question_count"]),
            reply_event_id=reply_event["reply_event_id"],
            raise_on_failure=False,
        )
        update_state_after_grading(state, reply_event, delivery_receipt, False, args.now)
        state["failure"] = {
            "event_type": "grading",
            "errors": [str(error), *delivery_receipt.get("errors", [])],
            "at_unix_seconds": state["updated_at_unix_seconds"],
        }

    write_json(args.run_state, state)
    if delivery_receipt["status"] not in {"dry_run_ready", "sent"}:
        raise RuntimeError("; ".join(delivery_receipt["errors"]) or delivery_receipt["status"])


def command_generate(args: argparse.Namespace) -> None:
    schedule = load_json(args.study_schedule)
    topic_map = load_json(args.topic_map)
    question_bank = load_json(args.question_bank) if args.question_bank else None
    quiz_set = generate_quiz_set(schedule, topic_map, args.date, args.question_count, question_bank)
    write_json(args.out, quiz_set)


def command_adaptive_plan(args: argparse.Namespace) -> None:
    schedule = load_json(args.study_schedule)
    topic_map = load_json(args.topic_map)
    question_bank = load_json(args.question_bank) if args.question_bank else None
    grading_reports = [load_json(path) for path in args.grading_report]
    adaptive_plan = build_adaptive_plan(
        schedule,
        topic_map,
        question_bank,
        grading_reports,
        args.date,
        args.question_count,
        args.weak_topic_threshold,
    )
    write_json(args.out, adaptive_plan)


def command_fixture_submission(args: argparse.Namespace) -> None:
    quiz_set = load_json(args.quiz_set)
    write_json(args.out, generate_fixture_submission(quiz_set))


def command_slack_reply_fixture(args: argparse.Namespace) -> None:
    quiz_set = load_json(args.quiz_set)
    submission = generate_fixture_submission(quiz_set)
    reply_event = build_slack_reply_event_fixture(quiz_set, submission)
    write_json(args.out, reply_event)


def command_socket_payload_fixture(args: argparse.Namespace) -> None:
    quiz_set = load_json(args.quiz_set)
    submission = generate_fixture_submission(quiz_set)
    socket_payload = build_socket_mode_payload_fixture(quiz_set, submission)
    write_json(args.out, socket_payload)


def command_grade(args: argparse.Namespace) -> None:
    quiz_set = load_json(args.quiz_set)
    submission = load_json(args.answer_submission)
    grading_report, recommendation = grade_submission(quiz_set, submission)
    write_json(args.grading_report, grading_report)
    write_json(args.study_recommendation, recommendation)


def command_reply_intake(args: argparse.Namespace) -> None:
    quiz_set = load_json(args.quiz_set)
    reply_event = load_json(args.slack_reply_event)
    write_slack_reply_grading_artifacts(quiz_set, reply_event, args.out_dir)


def command_socket_reply_intake(args: argparse.Namespace) -> None:
    load_env_file(getattr(args, "env_file", None))
    started_at = int(time.time())
    quiz_set = load_json(args.quiz_set)
    socket_payload = load_json(args.socket_payload)
    event_payload = socket_payload.get("payload", socket_payload)
    mode = getattr(args, "mode", "fixture")
    run_state_path = getattr(args, "run_state", None)
    expected_channel_id = args.expected_channel_id
    expected_thread_ts = args.expected_thread_ts
    env_readiness = slack_env_readiness(require_app_token=True)
    if run_state_path:
        state = load_json(run_state_path)
        expected_channel_id, expected_thread_ts = resolve_run_state_reply_target(
            state,
            expected_channel_id,
            expected_thread_ts,
        )
        if not expected_thread_ts:
            errors = ["run_state slack.thread_ts is required before socket reply grading"]
            write_run_state_socket_reply_failure(
                quiz_set,
                args.out_dir,
                run_state_path,
                mode,
                "run_state_thread_missing",
                env_readiness,
                False,
                started_at,
                getattr(args, "now", None),
                None,
                errors,
            )
            raise RuntimeError("; ".join(errors))

    reply_event = build_slack_reply_event_from_socket_payload(
        quiz_set,
        event_payload,
        expected_channel_id,
        expected_thread_ts,
        socket_payload.get("envelope_id"),
    )
    if reply_event is None:
        receipt = build_slack_reply_intake_receipt(
            quiz_set,
            "fixture" if not run_state_path else mode,
            "ignored_event",
            env_readiness,
            False,
            False,
            started_at,
            errors=["socket payload was not a matching Slack thread reply event"],
        )
        write_json(args.out_dir / "slack_reply_intake_receipt.json", receipt)
        raise RuntimeError("socket payload was not a matching Slack thread reply event")

    if run_state_path:
        receipt = process_slack_reply_event_with_run_state(
            quiz_set,
            reply_event,
            args.out_dir,
            run_state_path,
            mode,
            env_readiness,
            False,
            started_at,
            getattr(args, "now", None),
        )
        if receipt["status"] not in SOCKET_REPLY_LISTEN_SUCCESS_STATUSES:
            raise RuntimeError("; ".join(receipt.get("errors", [])) or receipt["status"])
        return

    artifacts = write_slack_reply_grading_artifacts(quiz_set, reply_event, args.out_dir)
    receipt = build_slack_reply_intake_receipt(
        quiz_set,
        "fixture",
        "fixture_received_and_graded",
        slack_env_readiness({}, require_app_token=True),
        False,
        False,
        started_at,
        reply_event_id=reply_event["reply_event_id"],
        grading_response_id=artifacts["slack_grading_response"]["response_id"],
    )
    write_json(args.out_dir / "slack_reply_intake_receipt.json", receipt)


def command_quiz_prompt(args: argparse.Namespace) -> None:
    quiz_set = load_json(args.quiz_set)
    quiz_prompt = build_quiz_prompt(quiz_set, args.source_quiz_set_path)
    write_json(args.json_out, quiz_prompt)
    write_text(args.markdown_out, render_quiz_prompt_markdown(quiz_prompt))


def command_slack_smoke(args: argparse.Namespace) -> None:
    load_env_file(getattr(args, "env_file", None))
    quiz_set = load_json(args.quiz_set)
    write_slack_delivery_artifacts(
        quiz_set,
        args.out_dir,
        args.artifact_path,
        args.preview_count,
        args.mode,
    )


def command_slack_grading_send(args: argparse.Namespace) -> None:
    load_env_file(args.env_file)
    quiz_set = load_json(args.quiz_set)
    grading_response = load_json(args.slack_grading_response)
    thread_ts = args.thread_ts or grading_response["thread_ts"]
    write_slack_grading_delivery_receipt(
        quiz_set,
        args.out_dir,
        args.mode,
        "grading_result",
        thread_ts,
        grading_response["response_text"],
        reply_event_id=grading_response.get("reply_event_id"),
        grading_response_id=grading_response.get("response_id"),
    )


def command_socket_reply_listen(args: argparse.Namespace) -> None:
    if args.mode == "live" and not args.expected_thread_ts and not args.run_state:
        raise RuntimeError("--expected-thread-ts is required in live mode")

    loaded_env = load_env_file(args.env_file)
    started_at = int(time.time())
    quiz_set = load_json(args.quiz_set)
    env_readiness = slack_env_readiness(require_app_token=True)
    expected_channel_id = args.expected_channel_id or os.environ.get(SLACK_CHANNEL_ENV)
    expected_thread_ts = args.expected_thread_ts

    if args.run_state:
        state = load_json(args.run_state)
        expected_channel_id, expected_thread_ts = resolve_run_state_reply_target(
            state,
            args.expected_channel_id,
            args.expected_thread_ts,
        )
        if args.mode == "live" and not expected_thread_ts:
            errors = ["run_state slack.thread_ts is required before socket reply grading"]
            write_run_state_socket_reply_failure(
                quiz_set,
                args.out_dir,
                args.run_state,
                args.mode,
                "run_state_thread_missing",
                env_readiness,
                True,
                started_at,
                args.now,
                args.timeout_seconds,
                errors,
            )
            raise RuntimeError("; ".join(errors))

    if args.mode == "dry-run":
        receipt = build_slack_reply_intake_receipt(
            quiz_set,
            args.mode,
            "dry_run_ready",
            env_readiness,
            False,
            False,
            started_at,
            timeout_seconds=args.timeout_seconds,
            errors=[] if not loaded_env else [],
        )
        write_json(args.out_dir / "slack_reply_intake_receipt.json", receipt)
        return

    errors = [f"missing {name}" for name in env_readiness["missing_env"]]
    if not env_readiness["slack_sdk_available"]:
        errors.append("slack_sdk is not installed")
    if not env_readiness["socket_mode_sdk_available"]:
        errors.append("slack_sdk.socket_mode is not available")
    if errors:
        receipt = build_slack_reply_intake_receipt(
            quiz_set,
            args.mode,
            "blocked_missing_env_or_sdk",
            env_readiness,
            False,
            False,
            started_at,
            timeout_seconds=args.timeout_seconds,
            errors=errors,
        )
        write_json(args.out_dir / "slack_reply_intake_receipt.json", receipt)
        raise RuntimeError("; ".join(errors))

    try:
        from slack_sdk.socket_mode import SocketModeClient
        from slack_sdk.socket_mode.response import SocketModeResponse
    except ModuleNotFoundError as error:
        receipt = build_slack_reply_intake_receipt(
            quiz_set,
            args.mode,
            "blocked_missing_env_or_sdk",
            env_readiness,
            False,
            False,
            started_at,
            timeout_seconds=args.timeout_seconds,
            errors=[str(error)],
        )
        write_json(args.out_dir / "slack_reply_intake_receipt.json", receipt)
        raise RuntimeError("slack_sdk socket mode dependency is missing") from error

    done = threading.Event()
    result: dict[str, Any] = {}
    client = SocketModeClient(app_token=os.environ[SLACK_APP_TOKEN_ENV])

    def process(client: Any, req: Any) -> None:
        if req.type != "events_api":
            return

        client.send_socket_mode_response(SocketModeResponse(envelope_id=req.envelope_id))
        reply_event = build_slack_reply_event_from_socket_payload(
            quiz_set,
            req.payload,
            expected_channel_id,
            expected_thread_ts,
            req.envelope_id,
        )
        if reply_event is None:
            return

        if args.run_state:
            receipt = process_slack_reply_event_with_run_state(
                quiz_set,
                reply_event,
                args.out_dir,
                args.run_state,
                args.mode,
                env_readiness,
                True,
                started_at,
                args.now,
                timeout_seconds=args.timeout_seconds,
            )
            result.update(
                {
                    "status": receipt["status"],
                    "reply_event_id": receipt["received_reply_event_id"],
                    "grading_response_id": receipt["grading_response_id"],
                    "slack_response": receipt["slack_response"],
                    "slack_used": receipt["slack_used"],
                    "errors": receipt["errors"],
                }
            )
            done.set()
            return

        try:
            artifacts = write_slack_reply_grading_artifacts(quiz_set, reply_event, args.out_dir)
            delivery_receipt = None
            if args.send_thread_response:
                delivery_receipt = write_slack_grading_delivery_receipt(
                    quiz_set,
                    args.out_dir,
                    args.mode,
                    "grading_result",
                    reply_event["thread_ts"],
                    artifacts["slack_grading_response"]["response_text"],
                    reply_event_id=reply_event["reply_event_id"],
                    grading_response_id=artifacts["slack_grading_response"]["response_id"],
                    raise_on_failure=False,
                )
            send_succeeded = delivery_receipt is None or delivery_receipt["status"] == "sent"
            result.update(
                {
                    "status": (
                        SOCKET_REPLY_LISTEN_SENT_STATUS
                        if args.send_thread_response and send_succeeded
                        else (
                            "grading_response_send_failed"
                            if args.send_thread_response
                            else SOCKET_REPLY_LISTEN_SUCCESS_STATUS
                        )
                    ),
                    "reply_event_id": reply_event["reply_event_id"],
                    "grading_response_id": artifacts["slack_grading_response"]["response_id"],
                    "slack_response": delivery_receipt["slack_response"] if delivery_receipt else None,
                    "slack_used": bool(delivery_receipt and delivery_receipt["slack_used"]),
                    "errors": [] if send_succeeded else delivery_receipt["errors"],
                }
            )
        except Exception as error:  # pragma: no cover - exercised by live Slack smoke.
            delivery_receipt = None
            if args.send_thread_response:
                delivery_receipt = write_slack_grading_delivery_receipt(
                    quiz_set,
                    args.out_dir,
                    args.mode,
                    "invalid_reply_error",
                    reply_event["thread_ts"],
                    build_invalid_reply_error_text(error, quiz_set["question_count"]),
                    reply_event_id=reply_event["reply_event_id"],
                    raise_on_failure=False,
                )
            send_succeeded = delivery_receipt is not None and delivery_receipt["status"] == "sent"
            result.update(
                {
                    "status": (
                        SOCKET_REPLY_LISTEN_INVALID_SENT_STATUS
                        if send_succeeded
                        else (
                            "invalid_reply_error_send_failed"
                            if args.send_thread_response
                            else "received_invalid_reply"
                        )
                    ),
                    "reply_event_id": reply_event["reply_event_id"],
                    "grading_response_id": None,
                    "slack_response": delivery_receipt["slack_response"] if delivery_receipt else None,
                    "slack_used": bool(delivery_receipt and delivery_receipt["slack_used"]),
                    "errors": [str(error), *(delivery_receipt["errors"] if delivery_receipt else [])],
                }
            )
        finally:
            done.set()

    client.socket_mode_request_listeners.append(process)
    try:
        client.connect()
        if not done.wait(args.timeout_seconds):
            result.update(
                {
                    "status": "timeout_no_reply",
                    "reply_event_id": None,
                    "grading_response_id": None,
                    "slack_response": None,
                    "errors": [f"no matching Slack thread reply event within {args.timeout_seconds} seconds"],
                }
            )
    except Exception as error:
        result.update(
            {
                "status": "connection_failed",
                "reply_event_id": None,
                "grading_response_id": None,
                "slack_response": None,
                "errors": [str(error)],
            }
        )
    finally:
        client.close()

    receipt = build_slack_reply_intake_receipt(
        quiz_set,
        args.mode,
        result["status"],
        env_readiness,
        True,
        False,
        started_at,
        timeout_seconds=args.timeout_seconds,
        reply_event_id=result.get("reply_event_id"),
        grading_response_id=result.get("grading_response_id"),
        slack_response=result.get("slack_response"),
        errors=result.get("errors", []),
    )
    receipt["slack_used"] = bool(result.get("slack_used", False))
    write_json(args.out_dir / "slack_reply_intake_receipt.json", receipt)
    if result["status"] not in SOCKET_REPLY_LISTEN_SUCCESS_STATUSES:
        raise RuntimeError("; ".join(result.get("errors", [])) or result["status"])


def command_run_fixture(args: argparse.Namespace) -> None:
    started_at = int(time.time())
    schedule = load_json(args.study_schedule)
    topic_map = load_json(args.topic_map)
    question_bank = load_json(args.question_bank) if args.question_bank else None
    quiz_set = generate_quiz_set(schedule, topic_map, args.date, args.question_count, question_bank)
    fixture_submission = generate_fixture_submission(quiz_set)
    socket_payload = build_socket_mode_payload_fixture(quiz_set, fixture_submission)
    slack_reply_event = build_slack_reply_event_from_socket_payload(
        quiz_set,
        socket_payload["payload"],
        socket_envelope_id=socket_payload["envelope_id"],
    )
    if slack_reply_event is None:
        raise RuntimeError("generated socket payload did not produce a Slack reply event")
    quiz_prompt = build_quiz_prompt(quiz_set, str(args.out_dir / "quiz_set.json"))
    write_json(args.out_dir / "quiz_set.json", quiz_set)
    write_json(args.out_dir / "quiz_prompt.json", quiz_prompt)
    write_text(args.out_dir / "quiz_prompt.md", render_quiz_prompt_markdown(quiz_prompt))
    write_json(args.out_dir / "slack_socket_mode_payload.json", socket_payload)
    artifacts = write_slack_reply_grading_artifacts(quiz_set, slack_reply_event, args.out_dir)
    receipt = build_slack_reply_intake_receipt(
        quiz_set,
        "fixture",
        "fixture_received_and_graded",
        slack_env_readiness({}, require_app_token=True),
        False,
        False,
        started_at,
        reply_event_id=slack_reply_event["reply_event_id"],
        grading_response_id=artifacts["slack_grading_response"]["response_id"],
    )
    write_json(args.out_dir / "slack_reply_intake_receipt.json", receipt)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Run AICX study bot fixture quiz PoC")
    subparsers = parser.add_subparsers(dest="command", required=True)

    generate = subparsers.add_parser("generate")
    generate.add_argument("--study-schedule", type=Path, required=True)
    generate.add_argument("--topic-map", type=Path, required=True)
    generate.add_argument("--question-bank", type=Path)
    generate.add_argument("--date", required=True)
    generate.add_argument("--question-count", type=int)
    generate.add_argument("--out", type=Path, required=True)
    generate.set_defaults(func=command_generate)

    adaptive_plan = subparsers.add_parser("adaptive-plan")
    adaptive_plan.add_argument("--study-schedule", type=Path, required=True)
    adaptive_plan.add_argument("--topic-map", type=Path, required=True)
    adaptive_plan.add_argument("--question-bank", type=Path)
    adaptive_plan.add_argument("--grading-report", type=Path, action="append", required=True)
    adaptive_plan.add_argument("--date", required=True)
    adaptive_plan.add_argument("--question-count", type=int)
    adaptive_plan.add_argument("--weak-topic-threshold", type=float, default=DEFAULT_WEAK_TOPIC_THRESHOLD)
    adaptive_plan.add_argument("--out", type=Path, required=True)
    adaptive_plan.set_defaults(func=command_adaptive_plan)

    fixture_submission = subparsers.add_parser("fixture-submission")
    fixture_submission.add_argument("--quiz-set", type=Path, required=True)
    fixture_submission.add_argument("--out", type=Path, required=True)
    fixture_submission.set_defaults(func=command_fixture_submission)

    slack_reply_fixture = subparsers.add_parser("slack-reply-fixture")
    slack_reply_fixture.add_argument("--quiz-set", type=Path, required=True)
    slack_reply_fixture.add_argument("--out", type=Path, required=True)
    slack_reply_fixture.set_defaults(func=command_slack_reply_fixture)

    socket_payload_fixture = subparsers.add_parser("socket-payload-fixture")
    socket_payload_fixture.add_argument("--quiz-set", type=Path, required=True)
    socket_payload_fixture.add_argument("--out", type=Path, required=True)
    socket_payload_fixture.set_defaults(func=command_socket_payload_fixture)

    grade = subparsers.add_parser("grade")
    grade.add_argument("--quiz-set", type=Path, required=True)
    grade.add_argument("--answer-submission", type=Path, required=True)
    grade.add_argument("--grading-report", type=Path, required=True)
    grade.add_argument("--study-recommendation", type=Path, required=True)
    grade.set_defaults(func=command_grade)

    reply_intake = subparsers.add_parser("reply-intake")
    reply_intake.add_argument("--quiz-set", type=Path, required=True)
    reply_intake.add_argument("--slack-reply-event", type=Path, required=True)
    reply_intake.add_argument("--out-dir", type=Path, required=True)
    reply_intake.set_defaults(func=command_reply_intake)

    socket_reply_intake = subparsers.add_parser("socket-reply-intake")
    socket_reply_intake.add_argument("--quiz-set", type=Path, required=True)
    socket_reply_intake.add_argument("--socket-payload", type=Path, required=True)
    socket_reply_intake.add_argument("--out-dir", type=Path, required=True)
    socket_reply_intake.add_argument("--run-state", type=Path)
    socket_reply_intake.add_argument("--mode", choices=["dry-run", "live"], default="dry-run")
    socket_reply_intake.add_argument("--expected-channel-id")
    socket_reply_intake.add_argument("--expected-thread-ts")
    socket_reply_intake.add_argument("--env-file", type=Path)
    socket_reply_intake.add_argument("--now")
    socket_reply_intake.set_defaults(func=command_socket_reply_intake)

    quiz_prompt = subparsers.add_parser("quiz-prompt")
    quiz_prompt.add_argument("--quiz-set", type=Path, required=True)
    quiz_prompt.add_argument("--json-out", type=Path, required=True)
    quiz_prompt.add_argument("--markdown-out", type=Path, required=True)
    quiz_prompt.add_argument("--source-quiz-set-path")
    quiz_prompt.set_defaults(func=command_quiz_prompt)

    slack_smoke = subparsers.add_parser("slack-smoke")
    slack_smoke.add_argument("--quiz-set", type=Path, required=True)
    slack_smoke.add_argument("--out-dir", type=Path, required=True)
    slack_smoke.add_argument("--artifact-path", required=True)
    slack_smoke.add_argument("--preview-count", type=int, default=3)
    slack_smoke.add_argument("--mode", choices=["dry-run", "live"], default="dry-run")
    slack_smoke.add_argument("--env-file", type=Path)
    slack_smoke.set_defaults(func=command_slack_smoke)

    slack_grading_send = subparsers.add_parser("slack-grading-send")
    slack_grading_send.add_argument("--quiz-set", type=Path, required=True)
    slack_grading_send.add_argument("--slack-grading-response", type=Path, required=True)
    slack_grading_send.add_argument("--out-dir", type=Path, required=True)
    slack_grading_send.add_argument("--mode", choices=["dry-run", "live"], default="dry-run")
    slack_grading_send.add_argument("--thread-ts")
    slack_grading_send.add_argument("--env-file", type=Path)
    slack_grading_send.set_defaults(func=command_slack_grading_send)

    daily_dispatch = subparsers.add_parser("daily-dispatch")
    daily_dispatch.add_argument("--study-schedule", type=Path, required=True)
    daily_dispatch.add_argument("--topic-map", type=Path, required=True)
    daily_dispatch.add_argument("--question-bank", type=Path)
    daily_dispatch.add_argument("--adaptive-plan", type=Path)
    daily_dispatch.add_argument("--date", required=True)
    daily_dispatch.add_argument("--question-count", type=int)
    daily_dispatch.add_argument("--out-dir", type=Path, required=True)
    daily_dispatch.add_argument("--run-state", type=Path)
    daily_dispatch.add_argument("--mode", choices=["dry-run", "live"], default="dry-run")
    daily_dispatch.add_argument("--preview-count", type=int, default=3)
    daily_dispatch.add_argument("--env-file", type=Path)
    daily_dispatch.add_argument("--now")
    daily_dispatch.add_argument("--force", action="store_true")
    daily_dispatch.add_argument("--allow-late", action="store_true")
    daily_dispatch.add_argument("--late-window-hours", type=int, default=12)
    daily_dispatch.set_defaults(func=command_daily_dispatch)

    daily_grade = subparsers.add_parser("daily-grade")
    daily_grade.add_argument("--run-state", type=Path, required=True)
    daily_grade.add_argument("--quiz-set", type=Path, required=True)
    daily_grade.add_argument("--slack-reply-event", type=Path, required=True)
    daily_grade.add_argument("--out-dir", type=Path, required=True)
    daily_grade.add_argument("--mode", choices=["dry-run", "live"], default="dry-run")
    daily_grade.add_argument("--env-file", type=Path)
    daily_grade.add_argument("--now")
    daily_grade.set_defaults(func=command_daily_grade)

    daily_grade_poll = subparsers.add_parser("daily-grade-poll")
    daily_grade_poll.add_argument("--run-state", type=Path, required=True)
    daily_grade_poll.add_argument("--quiz-set", type=Path)
    daily_grade_poll.add_argument("--out-dir", type=Path)
    daily_grade_poll.add_argument("--mode", choices=["dry-run", "live"], default="dry-run")
    daily_grade_poll.add_argument("--thread-messages-fixture", type=Path)
    daily_grade_poll.add_argument("--env-file", type=Path)
    daily_grade_poll.add_argument("--now")
    daily_grade_poll.set_defaults(func=command_daily_grade_poll)

    daily_maintain = subparsers.add_parser("daily-maintain")
    daily_maintain.add_argument("--date", required=True)
    daily_maintain.add_argument("--out-root", type=Path, required=True)
    daily_maintain.add_argument("--study-schedule", type=Path, default=DEFAULT_STUDY_SCHEDULE_PATH)
    daily_maintain.add_argument("--topic-map", type=Path, default=DEFAULT_TOPIC_MAP_PATH)
    daily_maintain.add_argument("--question-bank", type=Path, default=DEFAULT_QUESTION_BANK_PATH)
    daily_maintain.add_argument("--adaptive-plan", type=Path)
    daily_maintain.add_argument("--question-count", type=int)
    daily_maintain.add_argument("--mode", choices=["dry-run", "live"], default="live")
    daily_maintain.add_argument("--preview-count", type=int, default=3)
    daily_maintain.add_argument("--thread-messages-fixture", type=Path)
    daily_maintain.add_argument("--env-file", type=Path)
    daily_maintain.add_argument("--now")
    daily_maintain.add_argument("--allow-late", action="store_true")
    daily_maintain.add_argument("--late-window-hours", type=int, default=12)
    daily_maintain.set_defaults(func=command_daily_maintain)

    daily_status = subparsers.add_parser("daily-status")
    daily_status.add_argument("--run-state", type=Path, required=True)
    daily_status.set_defaults(func=command_daily_status)

    socket_reply_listen = subparsers.add_parser("socket-reply-listen")
    socket_reply_listen.add_argument("--quiz-set", type=Path, required=True)
    socket_reply_listen.add_argument("--out-dir", type=Path, required=True)
    socket_reply_listen.add_argument("--mode", choices=["dry-run", "live"], default="dry-run")
    socket_reply_listen.add_argument("--timeout-seconds", type=int, default=60)
    socket_reply_listen.add_argument("--run-state", type=Path)
    socket_reply_listen.add_argument("--single-run", action="store_true")
    socket_reply_listen.add_argument("--expected-channel-id")
    socket_reply_listen.add_argument("--expected-thread-ts")
    socket_reply_listen.add_argument("--send-thread-response", action="store_true")
    socket_reply_listen.add_argument("--env-file", type=Path)
    socket_reply_listen.add_argument("--now")
    socket_reply_listen.set_defaults(func=command_socket_reply_listen)

    run_fixture = subparsers.add_parser("run-fixture")
    run_fixture.add_argument("--study-schedule", type=Path, required=True)
    run_fixture.add_argument("--topic-map", type=Path, required=True)
    run_fixture.add_argument("--question-bank", type=Path)
    run_fixture.add_argument("--date", required=True)
    run_fixture.add_argument("--question-count", type=int)
    run_fixture.add_argument("--out-dir", type=Path, required=True)
    run_fixture.set_defaults(func=command_run_fixture)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    args.func(args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
