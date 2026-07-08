import io
import importlib.util
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch


REPO_ROOT = Path(__file__).resolve().parents[1]
FIXTURE_DIR = REPO_ROOT / "docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot"
MODULE_PATH = REPO_ROOT / "scripts/aicx_study_fixture.py"

spec = importlib.util.spec_from_file_location("aicx_study_fixture", MODULE_PATH)
aicx_study_fixture = importlib.util.module_from_spec(spec)
assert spec and spec.loader
sys.modules[spec.name] = aicx_study_fixture
spec.loader.exec_module(aicx_study_fixture)


class AicxStudyFixtureTest(unittest.TestCase):
    def setUp(self):
        self.schedule = aicx_study_fixture.load_json(FIXTURE_DIR / "study_schedule.json")
        self.topic_map = aicx_study_fixture.load_json(FIXTURE_DIR / "topic_map.json")
        self.question_bank = aicx_study_fixture.load_json(FIXTURE_DIR / "question_bank.fixture.json")

    def make_daily_maintain_args(
        self,
        tmp_dir: Path,
        now: str,
        *,
        mode: str = "dry-run",
        allow_late: bool = False,
        thread_messages_fixture: Path | None = None,
    ):
        return type(
            "Args",
            (),
            {
                "date": "2026-06-21",
                "out_root": tmp_dir,
                "study_schedule": FIXTURE_DIR / "study_schedule.json",
                "topic_map": FIXTURE_DIR / "topic_map.json",
                "question_bank": FIXTURE_DIR / "question_bank.fixture.json",
                "adaptive_plan": None,
                "question_count": None,
                "mode": mode,
                "preview_count": 3,
                "thread_messages_fixture": thread_messages_fixture,
                "env_file": None,
                "now": now,
                "allow_late": allow_late,
                "late_window_hours": 12,
            },
        )()

    def test_generate_quiz_set_resolves_2026_06_21_scope(self):
        quiz_set = aicx_study_fixture.generate_quiz_set(
            self.schedule,
            self.topic_map,
            "2026-06-21",
            30,
            self.question_bank,
        )

        self.assertEqual("QUIZ-ASB-2026-06-21", quiz_set["quiz_set_id"])
        self.assertEqual("WEEK-ASB-2026-06-17", quiz_set["study_window"]["week_id"])
        self.assertEqual([{"from": 258, "to": 368}], quiz_set["study_window"]["study_pages"])
        self.assertEqual(30, quiz_set["question_count"])
        self.assertEqual(30, len(quiz_set["questions"]))
        self.assertFalse(quiz_set["pdf_ingest_used"])
        self.assertFalse(quiz_set["slack_used"])
        self.assertEqual("multiple_choice_a_d", quiz_set["answer_format"])
        self.assertIn("TOPIC-ORG-KPI", {topic["topic_id"] for topic in quiz_set["topic_scope"]})
        self.assertIn("TOPIC-5D", {topic["topic_id"] for topic in quiz_set["topic_scope"]})
        bank_questions = [
            question for question in quiz_set["questions"]
            if question["source"] == "question_bank_fixture"
        ]
        self.assertEqual(10, len(bank_questions))
        self.assertEqual("QB-ASB-BRANCHING-001", bank_questions[0]["question_id"])
        self.assertIn("scenario_tags", bank_questions[0])

    def test_generate_quiz_set_defaults_to_ten_daily_questions(self):
        quiz_set = aicx_study_fixture.generate_quiz_set(
            self.schedule,
            self.topic_map,
            "2026-06-21",
            None,
            self.question_bank,
        )

        self.assertEqual(10, quiz_set["question_count"])
        self.assertEqual(10, len(quiz_set["questions"]))
        self.assertTrue(all(question["source"] == "question_bank_fixture" for question in quiz_set["questions"]))

    def test_fixture_submission_grades_topic_accuracy_and_pages(self):
        quiz_set = aicx_study_fixture.generate_quiz_set(
            self.schedule,
            self.topic_map,
            "2026-06-21",
            30,
            self.question_bank,
        )
        submission = aicx_study_fixture.generate_fixture_submission(quiz_set)
        grading_report, recommendation = aicx_study_fixture.grade_submission(quiz_set, submission)

        self.assertEqual(30, grading_report["total_questions"])
        self.assertEqual(24, grading_report["correct_count"])
        topic_accuracy = {
            item["topic_id"]: item["accuracy"]
            for item in grading_report["topic_results"]
        }
        self.assertEqual(0.5, topic_accuracy["TOPIC-ORG-KPI"])
        self.assertEqual(0.5, topic_accuracy["TOPIC-5D"])
        recommended_ranges = [
            page_range
            for item in recommendation["recommended_pages"]
            for page_range in item["page_ranges"]
        ]
        self.assertIn({"from": 312, "to": 327}, recommended_ranges)
        self.assertIn({"from": 351, "to": 368}, recommended_ranges)
        self.assertFalse(recommendation["pdf_ingest_required"])
        self.assertFalse(recommendation["slack_required"])

    def test_slack_reply_intake_parses_answers_and_builds_grading_response(self):
        quiz_set = aicx_study_fixture.generate_quiz_set(
            self.schedule,
            self.topic_map,
            "2026-06-21",
            30,
            self.question_bank,
        )
        fixture_submission = aicx_study_fixture.generate_fixture_submission(quiz_set)
        reply_event = aicx_study_fixture.build_slack_reply_event_fixture(
            quiz_set,
            fixture_submission,
        )
        submission = aicx_study_fixture.build_submission_from_slack_reply(quiz_set, reply_event)
        grading_report, recommendation = aicx_study_fixture.grade_submission(quiz_set, submission)
        response = aicx_study_fixture.build_slack_grading_response(
            quiz_set,
            reply_event,
            submission,
            grading_report,
            recommendation,
        )

        self.assertEqual("slack_reply_fixture_grading", submission["source"])
        self.assertEqual(reply_event["reply_event_id"], submission["source_event_id"])
        self.assertEqual(30, len(submission["answers"]))
        self.assertEqual(fixture_submission["answers"], submission["answers"])
        self.assertEqual(24, grading_report["correct_count"])
        self.assertIn("採点結果: 24/30 (80%)", response["response_text"])
        self.assertEqual(reply_event["thread_ts"], response["thread_ts"])
        self.assertFalse(response["slack_send_required"])
        recommended_ranges = [
            page_range
            for item in response["recommended_pages"]
            for page_range in item["page_ranges"]
        ]
        self.assertIn({"from": 312, "to": 327}, recommended_ranges)
        self.assertIn({"from": 351, "to": 368}, recommended_ranges)

    def test_socket_mode_payload_builds_reply_event_and_grades_submission(self):
        quiz_set = aicx_study_fixture.generate_quiz_set(
            self.schedule,
            self.topic_map,
            "2026-06-21",
            30,
            self.question_bank,
        )
        fixture_submission = aicx_study_fixture.generate_fixture_submission(quiz_set)
        socket_payload = aicx_study_fixture.build_socket_mode_payload_fixture(
            quiz_set,
            fixture_submission,
        )
        reply_event = aicx_study_fixture.build_slack_reply_event_from_socket_payload(
            quiz_set,
            socket_payload["payload"],
            "C_FIXTURE",
            "1782074738.865009",
            socket_payload["envelope_id"],
        )
        self.assertIsNotNone(reply_event)
        assert reply_event is not None
        submission = aicx_study_fixture.build_submission_from_slack_reply(quiz_set, reply_event)
        grading_report, recommendation = aicx_study_fixture.grade_submission(quiz_set, submission)
        response = aicx_study_fixture.build_slack_grading_response(
            quiz_set,
            reply_event,
            submission,
            grading_report,
            recommendation,
        )

        self.assertEqual("socket_mode_event", reply_event["source"])
        self.assertEqual(socket_payload["envelope_id"], reply_event["socket_envelope_id"])
        self.assertEqual("slack_socket_mode_grading", submission["source"])
        self.assertEqual(24, grading_report["correct_count"])
        self.assertIn("復習推奨ページ:", response["response_text"])

    def test_socket_mode_payload_ignores_non_thread_or_wrong_channel_messages(self):
        quiz_set = aicx_study_fixture.generate_quiz_set(
            self.schedule,
            self.topic_map,
            "2026-06-21",
            30,
            self.question_bank,
        )
        payload = {
            "event_id": "EV-IGNORED",
            "event": {
                "type": "message",
                "channel": "C_OTHER",
                "user": "U_FIXTURE",
                "text": "1:A",
                "ts": "1782074800.000000",
                "thread_ts": "1782074738.865009",
            },
        }
        self.assertIsNone(
            aicx_study_fixture.build_slack_reply_event_from_socket_payload(
                quiz_set,
                payload,
                expected_channel_id="C_FIXTURE",
            )
        )
        payload["event"]["thread_ts"] = "1782074738.865009"
        payload["event"]["ts"] = "1782074800.000000"
        payload["event"]["bot_id"] = "B_FIXTURE"
        self.assertIsNone(
            aicx_study_fixture.build_slack_reply_event_from_socket_payload(
                quiz_set,
                payload,
                expected_channel_id="C_FIXTURE",
            )
        )
        payload["event"]["channel"] = "C_FIXTURE"
        payload["event"]["thread_ts"] = payload["event"]["ts"]
        self.assertIsNone(
            aicx_study_fixture.build_slack_reply_event_from_socket_payload(
                quiz_set,
                payload,
                expected_channel_id="C_FIXTURE",
            )
        )

    def test_socket_mode_readiness_includes_app_token_when_required(self):
        readiness = aicx_study_fixture.slack_env_readiness(
            {
                "SLACK_BOT_TOKEN": "xoxb-test",
                "SLACK_CHANNEL_ID": "C_FIXTURE",
            },
            require_app_token=True,
        )

        self.assertTrue(readiness["bot_token_present"])
        self.assertTrue(readiness["channel_id_present"])
        self.assertFalse(readiness["app_token_present"])
        self.assertIn("SLACK_APP_TOKEN", readiness["missing_env"])

    def test_socket_reply_live_requires_expected_thread_ts(self):
        args = type(
            "Args",
            (),
            {
                "mode": "live",
                "expected_thread_ts": None,
                "env_file": None,
                "timeout_seconds": 1,
                "quiz_set": FIXTURE_DIR / "quiz_set.json",
                "out_dir": Path("/tmp/aicx-unused"),
                "run_state": None,
                "single_run": False,
                "expected_channel_id": None,
                "send_thread_response": False,
                "now": None,
            },
        )()

        with self.assertRaisesRegex(RuntimeError, "--expected-thread-ts is required"):
            aicx_study_fixture.command_socket_reply_listen(args)

    def test_slack_reply_answer_parser_accepts_common_delimiters(self):
        answers, duplicates = aicx_study_fixture.parse_slack_answer_text(
            "1:B Q2:c 3.D 4)A 5-A"
        )

        self.assertEqual({}, {key: value for key, value in answers.items() if key > 5})
        self.assertEqual({1: "B", 2: "C", 3: "D", 4: "A", 5: "A"}, answers)
        self.assertEqual([], duplicates)

    def test_slack_reply_intake_fails_closed_when_answers_are_missing(self):
        quiz_set = aicx_study_fixture.generate_quiz_set(
            self.schedule,
            self.topic_map,
            "2026-06-21",
            30,
            self.question_bank,
        )
        reply_event = aicx_study_fixture.build_slack_reply_event_fixture(
            quiz_set,
            aicx_study_fixture.generate_fixture_submission(quiz_set),
        )
        reply_event["text"] = "1:A 2:B"

        with self.assertRaisesRegex(ValueError, "missing=\\[3"):
            aicx_study_fixture.build_submission_from_slack_reply(quiz_set, reply_event)

    def test_invalid_reply_error_text_names_answer_format(self):
        text = aicx_study_fixture.build_invalid_reply_error_text("missing=[3]")

        self.assertIn("回答を採点できませんでした", text)
        self.assertIn("1:B 2:C", text)
        self.assertIn("missing=[3]", text)

    def test_slack_grading_delivery_receipt_dry_run(self):
        quiz_set = aicx_study_fixture.generate_quiz_set(
            self.schedule,
            self.topic_map,
            "2026-06-21",
            30,
            self.question_bank,
        )
        reply_event = aicx_study_fixture.build_slack_reply_event_fixture(
            quiz_set,
            aicx_study_fixture.generate_fixture_submission(quiz_set),
        )
        submission = aicx_study_fixture.build_submission_from_slack_reply(quiz_set, reply_event)
        grading_report, recommendation = aicx_study_fixture.grade_submission(quiz_set, submission)
        response = aicx_study_fixture.build_slack_grading_response(
            quiz_set,
            reply_event,
            submission,
            grading_report,
            recommendation,
        )

        with tempfile.TemporaryDirectory() as tmp_dir:
            receipt = aicx_study_fixture.write_slack_grading_delivery_receipt(
                quiz_set,
                Path(tmp_dir),
                "dry-run",
                "grading_result",
                response["thread_ts"],
                response["response_text"],
                reply_event_id=response["reply_event_id"],
                grading_response_id=response["response_id"],
            )

            self.assertEqual("aicx.slack_grading_delivery_receipt.v0", receipt["schema_version"])
            self.assertEqual("dry_run_ready", receipt["status"])
            self.assertEqual("grading_result", receipt["response_kind"])
            self.assertFalse(receipt["slack_used"])
            self.assertEqual(response["thread_ts"], receipt["thread_ts"])
            self.assertIn("採点結果", receipt["message_text"])
            self.assertTrue((Path(tmp_dir) / "slack_grading_delivery_receipt.json").exists())

    def test_invalid_reply_delivery_receipt_dry_run(self):
        quiz_set = aicx_study_fixture.generate_quiz_set(
            self.schedule,
            self.topic_map,
            "2026-06-21",
            30,
            self.question_bank,
        )
        error_text = aicx_study_fixture.build_invalid_reply_error_text("missing=[3]")

        with tempfile.TemporaryDirectory() as tmp_dir:
            receipt = aicx_study_fixture.write_slack_grading_delivery_receipt(
                quiz_set,
                Path(tmp_dir),
                "dry-run",
                "invalid_reply_error",
                "1782074738.865009",
                error_text,
                reply_event_id="SLACKREPLY-INVALID",
            )

            self.assertEqual("dry_run_ready", receipt["status"])
            self.assertEqual("invalid_reply_error", receipt["response_kind"])
            self.assertIsNone(receipt["grading_response_id"])
            self.assertIn("回答を採点できませんでした", receipt["message_text"])

    def test_build_slack_outbound_message_includes_all_daily_questions(self):
        quiz_set = aicx_study_fixture.generate_quiz_set(
            self.schedule,
            self.topic_map,
            "2026-06-21",
            None,
            self.question_bank,
        )
        message = aicx_study_fixture.build_slack_outbound_message(
            quiz_set,
            "docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/quiz_prompt.md",
            3,
        )

        self.assertEqual("aicx.slack_outbound_message.v0", message["schema_version"])
        self.assertEqual("slack", message["delivery_channel"])
        self.assertEqual(10, message["preview_count"])
        self.assertEqual(10, len(message["preview_questions"]))
        self.assertEqual(10, message["total_questions"])
        self.assertEqual(10, message["bank_question_count"])
        self.assertEqual(0, message["fallback_question_count"])
        self.assertIn("今日のAICX朝トレ", message["message_text"])
        self.assertIn("問題:", message["message_text"])
        self.assertIn("Q10.", message["message_text"])
        self.assertNotIn("問題プレビュー:", message["message_text"])
        self.assertIn("全文:", message["message_text"])
        self.assertNotIn("correct_choice", message["preview_questions"][0])
        self.assertNotIn("rationale", str(message["preview_questions"]))

    def test_build_quiz_prompt_excludes_answer_key_and_rationale(self):
        quiz_set = aicx_study_fixture.generate_quiz_set(
            self.schedule,
            self.topic_map,
            "2026-06-21",
            30,
            self.question_bank,
        )
        quiz_prompt = aicx_study_fixture.build_quiz_prompt(
            quiz_set,
            "docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/quiz_set.json",
        )
        rendered = aicx_study_fixture.render_quiz_prompt_markdown(quiz_prompt)
        question_serialized = str(quiz_prompt["questions"])

        self.assertEqual("aicx.quiz_prompt.v0", quiz_prompt["schema_version"])
        self.assertEqual("PROMPT-QUIZ-ASB-2026-06-21", quiz_prompt["quiz_prompt_id"])
        self.assertFalse(quiz_prompt["answer_key_included"])
        self.assertFalse(quiz_prompt["rationale_included"])
        self.assertEqual(30, quiz_prompt["question_count"])
        self.assertEqual(30, len(quiz_prompt["questions"]))
        self.assertNotIn("correct_choice", quiz_prompt["questions"][0])
        self.assertNotIn("rationale", quiz_prompt["questions"][0])
        self.assertNotIn("correct_choice", question_serialized)
        self.assertNotIn("rationale", question_serialized)
        self.assertIn("# AICX朝トレ 2026-06-21", rendered)
        self.assertIn("### Q30", rendered)
        self.assertNotIn("correct_choice", rendered)
        self.assertNotIn("rationale", rendered)

    def test_slack_delivery_receipt_dry_run_does_not_require_env_or_send(self):
        quiz_set = aicx_study_fixture.generate_quiz_set(
            self.schedule,
            self.topic_map,
            "2026-06-21",
            30,
            self.question_bank,
        )
        message = aicx_study_fixture.build_slack_outbound_message(
            quiz_set,
            "docs/standards/delivery-artifacts-v0/examples/aicx_ai_strategist_study_bot/quiz_set.json",
        )
        env_readiness = aicx_study_fixture.slack_env_readiness({})
        receipt = aicx_study_fixture.build_slack_delivery_receipt(
            message,
            "dry-run",
            env_readiness,
            "dry_run_ready",
            False,
        )

        self.assertEqual("aicx.slack_delivery_receipt.v0", receipt["schema_version"])
        self.assertEqual("dry-run", receipt["mode"])
        self.assertEqual("dry_run_ready", receipt["status"])
        self.assertFalse(receipt["slack_used"])
        self.assertIsNone(receipt["posted_at_unix_seconds"])
        self.assertFalse(receipt["env_readiness"]["bot_token_present"])
        self.assertFalse(receipt["env_readiness"]["channel_id_present"])
        self.assertEqual(["SLACK_BOT_TOKEN", "SLACK_CHANNEL_ID"], receipt["env_readiness"]["missing_env"])

    def test_daily_dispatch_waits_until_0430_jst(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            out_dir = Path(tmp_dir)
            args = type(
                "Args",
                (),
                {
                    "study_schedule": FIXTURE_DIR / "study_schedule.json",
                    "topic_map": FIXTURE_DIR / "topic_map.json",
                    "question_bank": FIXTURE_DIR / "question_bank.fixture.json",
                    "adaptive_plan": None,
                    "date": "2026-06-21",
                    "question_count": None,
                    "out_dir": out_dir,
                    "run_state": None,
                    "mode": "dry-run",
                    "preview_count": 3,
                    "env_file": None,
                    "now": "2026-06-21T04:29:00+09:00",
                    "force": False,
                },
            )()

            aicx_study_fixture.command_daily_dispatch(args)
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")

            self.assertEqual("aicx.daily_run_state.v0", state["schema_version"])
            self.assertEqual("not_due", state["status"])
            self.assertFalse(state["steps"]["quiz_generated"])
            self.assertFalse(state["steps"]["slack_sent"])
            self.assertEqual("not_due", state["last_event"]["status"])

    def test_daily_dispatch_writes_run_state_and_skips_duplicate(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            out_dir = Path(tmp_dir)
            args = type(
                "Args",
                (),
                {
                    "study_schedule": FIXTURE_DIR / "study_schedule.json",
                    "topic_map": FIXTURE_DIR / "topic_map.json",
                    "question_bank": FIXTURE_DIR / "question_bank.fixture.json",
                    "adaptive_plan": None,
                    "date": "2026-06-21",
                    "question_count": None,
                    "out_dir": out_dir,
                    "run_state": None,
                    "mode": "dry-run",
                    "preview_count": 3,
                    "env_file": None,
                    "now": "2026-06-21T04:30:00+09:00",
                    "force": False,
                },
            )()

            aicx_study_fixture.command_daily_dispatch(args)
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            quiz_set = aicx_study_fixture.load_json(out_dir / "quiz_set.json")
            receipt = aicx_study_fixture.load_json(out_dir / "slack_delivery_receipt.json")

            self.assertEqual("dispatched", state["status"])
            self.assertTrue(state["steps"]["quiz_generated"])
            self.assertTrue(state["steps"]["slack_sent"])
            self.assertFalse(state["slack"]["slack_used"])
            self.assertEqual("dry_run_ready", receipt["status"])
            self.assertEqual(10, quiz_set["question_count"])

            aicx_study_fixture.command_daily_dispatch(args)
            duplicate_state = aicx_study_fixture.load_json(out_dir / "run_state.json")

            self.assertTrue(duplicate_state["idempotency"]["duplicate_dispatch_skipped"])
            self.assertEqual("duplicate_skipped", duplicate_state["last_event"]["status"])

    def test_daily_dispatch_allow_late_marks_late_dispatch_and_message(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            out_dir = Path(tmp_dir)
            args = type(
                "Args",
                (),
                {
                    "study_schedule": FIXTURE_DIR / "study_schedule.json",
                    "topic_map": FIXTURE_DIR / "topic_map.json",
                    "question_bank": FIXTURE_DIR / "question_bank.fixture.json",
                    "adaptive_plan": None,
                    "date": "2026-06-21",
                    "question_count": None,
                    "out_dir": out_dir,
                    "run_state": None,
                    "mode": "dry-run",
                    "preview_count": 3,
                    "env_file": None,
                    "now": "2026-06-21T06:30:00+09:00",
                    "force": False,
                    "allow_late": True,
                    "late_window_hours": 12,
                },
            )()

            aicx_study_fixture.command_daily_dispatch(args)
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            message = aicx_study_fixture.load_json(out_dir / "slack_outbound_message.json")
            receipt = aicx_study_fixture.load_json(out_dir / "slack_delivery_receipt.json")

            self.assertEqual("dispatched", state["status"])
            self.assertEqual("late_dispatched", state["last_event"]["status"])
            self.assertIn("遅れて配信", message["message_text"])
            self.assertEqual("late_dispatched", receipt["dispatch_timing_status"])
            self.assertEqual(7200, receipt["late_by_seconds"])
            self.assertEqual(12, receipt["late_window_hours"])

    def test_daily_dispatch_allow_late_skips_when_late_window_expired(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            out_dir = Path(tmp_dir)
            args = type(
                "Args",
                (),
                {
                    "study_schedule": FIXTURE_DIR / "study_schedule.json",
                    "topic_map": FIXTURE_DIR / "topic_map.json",
                    "question_bank": FIXTURE_DIR / "question_bank.fixture.json",
                    "adaptive_plan": None,
                    "date": "2026-06-21",
                    "question_count": None,
                    "out_dir": out_dir,
                    "run_state": None,
                    "mode": "dry-run",
                    "preview_count": 3,
                    "env_file": None,
                    "now": "2026-06-21T18:31:00+09:00",
                    "force": False,
                    "allow_late": True,
                    "late_window_hours": 12,
                },
            )()

            aicx_study_fixture.command_daily_dispatch(args)
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")

            self.assertEqual("late_window_expired", state["status"])
            self.assertEqual("late_window_expired", state["last_event"]["status"])
            self.assertFalse(state["steps"]["slack_sent"])
            self.assertFalse((out_dir / "slack_delivery_receipt.json").exists())

    def test_daily_dispatch_allow_late_live_env_failure_does_not_mark_late_dispatched(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            out_dir = Path(tmp_dir)
            args = type(
                "Args",
                (),
                {
                    "study_schedule": FIXTURE_DIR / "study_schedule.json",
                    "topic_map": FIXTURE_DIR / "topic_map.json",
                    "question_bank": FIXTURE_DIR / "question_bank.fixture.json",
                    "adaptive_plan": None,
                    "date": "2026-06-21",
                    "question_count": None,
                    "out_dir": out_dir,
                    "run_state": None,
                    "mode": "live",
                    "preview_count": 3,
                    "env_file": None,
                    "now": "2026-06-21T06:30:00+09:00",
                    "force": False,
                    "allow_late": True,
                    "late_window_hours": 12,
                },
            )()

            with patch.dict(aicx_study_fixture.os.environ, {}, clear=True):
                with self.assertRaisesRegex(RuntimeError, "SLACK_BOT_TOKEN"):
                    aicx_study_fixture.command_daily_dispatch(args)

            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            receipt = aicx_study_fixture.load_json(out_dir / "slack_delivery_receipt.json")

            self.assertEqual("failed", state["status"])
            self.assertEqual("blocked_missing_env_or_sdk", state["last_event"]["status"])
            self.assertNotEqual("late_dispatched", state["last_event"]["status"])
            self.assertEqual("blocked_missing_env_or_sdk", receipt["status"])
            self.assertEqual("late_dispatched", receipt["dispatch_timing_status"])
            self.assertFalse(state["steps"]["slack_sent"])

    def test_daily_maintain_records_not_due_before_0430(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            args = self.make_daily_maintain_args(
                Path(tmp_dir),
                "2026-06-21T04:29:00+09:00",
            )

            aicx_study_fixture.command_daily_maintain(args)
            out_dir = Path(tmp_dir) / "2026-06-21"
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            receipt = aicx_study_fixture.load_json(out_dir / "maintenance_receipt.json")

            self.assertEqual("not_due", state["status"])
            self.assertEqual("not_due", receipt["status"])
            self.assertEqual("noop", receipt["action"])
            self.assertFalse(state["steps"]["slack_sent"])

    def test_daily_maintain_dispatches_after_0430(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            args = self.make_daily_maintain_args(
                Path(tmp_dir),
                "2026-06-21T04:30:00+09:00",
            )

            aicx_study_fixture.command_daily_maintain(args)
            out_dir = Path(tmp_dir) / "2026-06-21"
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            receipt = aicx_study_fixture.load_json(out_dir / "maintenance_receipt.json")
            delivery_receipt = aicx_study_fixture.load_json(out_dir / "slack_delivery_receipt.json")

            self.assertEqual("dispatched", state["status"])
            self.assertEqual("dispatched", receipt["status"])
            self.assertEqual("dispatch", receipt["action"])
            self.assertEqual("dry_run_ready", delivery_receipt["status"])
            self.assertTrue(state["steps"]["slack_sent"])

    def test_daily_maintain_late_dispatches_within_window(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            args = self.make_daily_maintain_args(
                Path(tmp_dir),
                "2026-06-21T06:30:00+09:00",
                allow_late=True,
            )

            aicx_study_fixture.command_daily_maintain(args)
            out_dir = Path(tmp_dir) / "2026-06-21"
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            receipt = aicx_study_fixture.load_json(out_dir / "maintenance_receipt.json")

            self.assertEqual("dispatched", state["status"])
            self.assertEqual("late_dispatched", state["last_event"]["status"])
            self.assertEqual("late_dispatched", receipt["status"])
            self.assertEqual("dispatch", receipt["action"])

    def test_daily_maintain_live_env_failure_records_maintenance_receipt(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            args = self.make_daily_maintain_args(
                Path(tmp_dir),
                "2026-06-21T06:30:00+09:00",
                mode="live",
                allow_late=True,
            )

            with patch.dict(aicx_study_fixture.os.environ, {}, clear=True):
                with self.assertRaisesRegex(RuntimeError, "SLACK_BOT_TOKEN"):
                    aicx_study_fixture.command_daily_maintain(args)

            out_dir = Path(tmp_dir) / "2026-06-21"
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            receipt = aicx_study_fixture.load_json(out_dir / "maintenance_receipt.json")

            self.assertEqual("failed", state["status"])
            self.assertEqual("blocked_missing_env_or_sdk", state["last_event"]["status"])
            self.assertEqual("blocked_missing_env_or_sdk", receipt["status"])
            self.assertNotEqual("late_dispatched", receipt["status"])

    def test_daily_dispatch_uses_adaptive_plan_when_provided(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            out_dir = Path(tmp_dir)
            args = type(
                "Args",
                (),
                {
                    "study_schedule": FIXTURE_DIR / "study_schedule.json",
                    "topic_map": FIXTURE_DIR / "topic_map.json",
                    "question_bank": FIXTURE_DIR / "question_bank.fixture.json",
                    "adaptive_plan": FIXTURE_DIR / "adaptive_plan.json",
                    "date": "2026-06-22",
                    "question_count": None,
                    "out_dir": out_dir,
                    "run_state": None,
                    "mode": "dry-run",
                    "preview_count": 3,
                    "env_file": None,
                    "now": "2026-06-22T04:30:00+09:00",
                    "force": False,
                },
            )()

            aicx_study_fixture.command_daily_dispatch(args)
            quiz_set = aicx_study_fixture.load_json(out_dir / "quiz_set.json")
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            message = aicx_study_fixture.load_json(out_dir / "slack_outbound_message.json")

            topic_counts = {}
            topic_sources = {}
            for question in quiz_set["questions"]:
                topic_counts[question["topic_id"]] = topic_counts.get(question["topic_id"], 0) + 1
                topic_sources.setdefault(question["topic_id"], []).append(question["source"])

            self.assertEqual("dispatched", state["status"])
            self.assertEqual(10, quiz_set["question_count"])
            self.assertEqual(10, message["preview_count"])
            self.assertEqual(10, len(message["preview_questions"]))
            self.assertIn("Q10.", message["message_text"])
            self.assertNotIn("問題プレビュー:", message["message_text"])
            self.assertEqual(4, topic_counts["TOPIC-5D"])
            self.assertEqual(3, topic_counts["TOPIC-ORG-KPI"])
            self.assertEqual(1, topic_counts["TOPIC-BRANCHING-CONTEXT"])
            self.assertEqual(1, topic_counts["TOPIC-WORKFLOW-CHECK"])
            self.assertEqual(1, topic_counts["TOPIC-ADOPTION-GOVERNANCE"])
            self.assertEqual(
                ["question_bank_fixture", "question_bank_fixture", "manual_fixture_topic_map", "manual_fixture_topic_map"],
                topic_sources["TOPIC-5D"],
            )
            self.assertEqual(
                ["question_bank_fixture", "question_bank_fixture", "manual_fixture_topic_map"],
                topic_sources["TOPIC-ORG-KPI"],
            )
            self.assertEqual(7, message["bank_question_count"])
            self.assertEqual(3, message["fallback_question_count"])
            self.assertTrue((out_dir / "adaptive_plan.json").exists())

    def test_adaptive_dispatch_fails_closed_on_date_mismatch(self):
        adaptive_plan = aicx_study_fixture.load_json(FIXTURE_DIR / "adaptive_plan.json")

        with self.assertRaisesRegex(ValueError, "next_quiz_date"):
            aicx_study_fixture.generate_quiz_set_from_adaptive_plan(
                self.schedule,
                self.topic_map,
                "2026-06-23",
                adaptive_plan,
                None,
                self.question_bank,
            )

    def test_adaptive_dispatch_fails_closed_on_question_count_mismatch(self):
        adaptive_plan = aicx_study_fixture.load_json(FIXTURE_DIR / "adaptive_plan.json")

        with self.assertRaisesRegex(ValueError, "question_count"):
            aicx_study_fixture.generate_quiz_set_from_adaptive_plan(
                self.schedule,
                self.topic_map,
                "2026-06-22",
                adaptive_plan,
                5,
                self.question_bank,
            )

    def test_adaptive_dispatch_fails_closed_on_allocation_sum_mismatch(self):
        adaptive_plan = aicx_study_fixture.load_json(FIXTURE_DIR / "adaptive_plan.json")
        adaptive_plan["topic_allocations"][0]["question_count"] += 1

        with self.assertRaisesRegex(ValueError, "topic_allocations total"):
            aicx_study_fixture.generate_quiz_set_from_adaptive_plan(
                self.schedule,
                self.topic_map,
                "2026-06-22",
                adaptive_plan,
                None,
                self.question_bank,
            )

    def test_daily_grade_updates_run_state_once(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            out_dir = Path(tmp_dir)
            dispatch_args = type(
                "Args",
                (),
                {
                    "study_schedule": FIXTURE_DIR / "study_schedule.json",
                    "topic_map": FIXTURE_DIR / "topic_map.json",
                    "question_bank": FIXTURE_DIR / "question_bank.fixture.json",
                    "adaptive_plan": None,
                    "date": "2026-06-21",
                    "question_count": None,
                    "out_dir": out_dir,
                    "run_state": None,
                    "mode": "dry-run",
                    "preview_count": 3,
                    "env_file": None,
                    "now": "2026-06-21T04:30:00+09:00",
                    "force": False,
                },
            )()
            aicx_study_fixture.command_daily_dispatch(dispatch_args)

            quiz_set = aicx_study_fixture.load_json(out_dir / "quiz_set.json")
            reply_event = aicx_study_fixture.build_slack_reply_event_fixture(
                quiz_set,
                aicx_study_fixture.generate_fixture_submission(quiz_set),
            )
            reply_path = out_dir / "slack_reply_event.json"
            aicx_study_fixture.write_json(reply_path, reply_event)
            grade_args = type(
                "Args",
                (),
                {
                    "run_state": out_dir / "run_state.json",
                    "quiz_set": out_dir / "quiz_set.json",
                    "slack_reply_event": reply_path,
                    "out_dir": out_dir,
                    "mode": "dry-run",
                    "env_file": None,
                    "now": "2026-06-21T04:40:00+09:00",
                },
            )()

            aicx_study_fixture.command_daily_grade(grade_args)
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            grading_report = aicx_study_fixture.load_json(out_dir / "grading_report.json")

            self.assertEqual("graded", state["status"])
            self.assertTrue(state["steps"]["reply_received"])
            self.assertTrue(state["steps"]["graded"])
            self.assertTrue(state["steps"]["grading_response_sent"])
            self.assertEqual([reply_event["reply_event_id"]], state["idempotency"]["processed_reply_event_ids"])
            self.assertEqual(10, grading_report["total_questions"])

            aicx_study_fixture.command_daily_grade(grade_args)
            duplicate_state = aicx_study_fixture.load_json(out_dir / "run_state.json")

            self.assertTrue(duplicate_state["idempotency"]["duplicate_grading_skipped"])
            self.assertEqual("duplicate_skipped", duplicate_state["last_event"]["status"])

    def test_socket_reply_intake_uses_run_state_thread_and_skips_duplicate(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            out_dir = Path(tmp_dir)
            dispatch_args = type(
                "Args",
                (),
                {
                    "study_schedule": FIXTURE_DIR / "study_schedule.json",
                    "topic_map": FIXTURE_DIR / "topic_map.json",
                    "question_bank": FIXTURE_DIR / "question_bank.fixture.json",
                    "adaptive_plan": None,
                    "date": "2026-06-21",
                    "question_count": None,
                    "out_dir": out_dir,
                    "run_state": None,
                    "mode": "dry-run",
                    "preview_count": 3,
                    "env_file": None,
                    "now": "2026-06-21T04:30:00+09:00",
                    "force": False,
                },
            )()
            aicx_study_fixture.command_daily_dispatch(dispatch_args)

            quiz_set = aicx_study_fixture.load_json(out_dir / "quiz_set.json")
            socket_payload = aicx_study_fixture.build_socket_mode_payload_fixture(
                quiz_set,
                aicx_study_fixture.generate_fixture_submission(quiz_set),
            )
            socket_payload_path = out_dir / "slack_socket_mode_payload.json"
            aicx_study_fixture.write_json(socket_payload_path, socket_payload)
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            state["slack"]["channel_id"] = "C_FIXTURE"
            state["slack"]["thread_ts"] = "1782074738.865009"
            aicx_study_fixture.write_json(out_dir / "run_state.json", state)

            intake_args = type(
                "Args",
                (),
                {
                    "quiz_set": out_dir / "quiz_set.json",
                    "socket_payload": socket_payload_path,
                    "out_dir": out_dir,
                    "run_state": out_dir / "run_state.json",
                    "mode": "dry-run",
                    "expected_channel_id": None,
                    "expected_thread_ts": None,
                    "env_file": None,
                    "now": "2026-06-21T04:45:00+09:00",
                },
            )()

            aicx_study_fixture.command_socket_reply_intake(intake_args)
            receipt = aicx_study_fixture.load_json(out_dir / "slack_reply_intake_receipt.json")
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")

            self.assertEqual("received_graded_and_sent", receipt["status"])
            self.assertEqual("graded", state["status"])
            self.assertTrue(state["steps"]["reply_received"])
            self.assertTrue(state["steps"]["graded"])
            self.assertTrue(state["steps"]["grading_response_sent"])
            self.assertEqual(receipt["received_reply_event_id"], state["idempotency"]["processed_reply_event_ids"][0])
            self.assertTrue((out_dir / "slack_reply_event.json").exists())
            self.assertTrue((out_dir / "slack_grading_delivery_receipt.json").exists())

            aicx_study_fixture.command_socket_reply_intake(intake_args)
            duplicate_receipt = aicx_study_fixture.load_json(out_dir / "slack_reply_intake_receipt.json")
            duplicate_state = aicx_study_fixture.load_json(out_dir / "run_state.json")

            self.assertEqual("duplicate_reply_skipped", duplicate_receipt["status"])
            self.assertTrue(duplicate_state["idempotency"]["duplicate_grading_skipped"])
            self.assertEqual("duplicate_reply_skipped", duplicate_state["last_event"]["status"])

    def test_daily_grade_poll_grades_unprocessed_thread_reply_once(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            out_dir = Path(tmp_dir)
            dispatch_args = type(
                "Args",
                (),
                {
                    "study_schedule": FIXTURE_DIR / "study_schedule.json",
                    "topic_map": FIXTURE_DIR / "topic_map.json",
                    "question_bank": FIXTURE_DIR / "question_bank.fixture.json",
                    "adaptive_plan": None,
                    "date": "2026-06-21",
                    "question_count": None,
                    "out_dir": out_dir,
                    "run_state": None,
                    "mode": "dry-run",
                    "preview_count": 3,
                    "env_file": None,
                    "now": "2026-06-21T04:30:00+09:00",
                    "force": False,
                },
            )()
            aicx_study_fixture.command_daily_dispatch(dispatch_args)

            quiz_set = aicx_study_fixture.load_json(out_dir / "quiz_set.json")
            submission = aicx_study_fixture.generate_fixture_submission(quiz_set)
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            state["slack"]["channel_id"] = "C_FIXTURE"
            state["slack"]["thread_ts"] = "1782074738.865009"
            state["status"] = "failed"
            state["failure"] = {
                "event_type": "thread_poll",
                "errors": ["temporary network failure"],
                "at_unix_seconds": 1782074700,
            }
            aicx_study_fixture.write_json(out_dir / "run_state.json", state)
            messages_fixture_path = out_dir / "thread_messages.json"
            aicx_study_fixture.write_json(
                messages_fixture_path,
                {
                    "messages": [
                        {
                            "type": "message",
                            "user": "U_BOT",
                            "bot_id": "B_FIXTURE",
                            "text": "採点結果",
                            "ts": "1782074750.000000",
                            "thread_ts": "1782074738.865009",
                        },
                        {
                            "type": "message",
                            "user": "U_PARENT",
                            "text": "今日の問題",
                            "ts": "1782074738.865009",
                        },
                        {
                            "type": "message",
                            "user": "U_FIXTURE",
                            "text": aicx_study_fixture.render_answer_submission_text(submission),
                            "ts": "1782074800.000000",
                            "thread_ts": "1782074738.865009",
                        },
                    ],
                },
            )
            poll_args = type(
                "Args",
                (),
                {
                    "run_state": out_dir / "run_state.json",
                    "quiz_set": None,
                    "out_dir": out_dir,
                    "mode": "dry-run",
                    "thread_messages_fixture": messages_fixture_path,
                    "env_file": None,
                    "now": "2026-06-21T04:45:00+09:00",
                },
            )()

            aicx_study_fixture.command_daily_grade_poll(poll_args)
            receipt = aicx_study_fixture.load_json(out_dir / "slack_thread_poll_receipt.json")
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            grading_report = aicx_study_fixture.load_json(out_dir / "grading_report.json")

            self.assertEqual("aicx.slack_thread_poll_receipt.v0", receipt["schema_version"])
            self.assertEqual("reply_found_graded_and_sent", receipt["status"])
            self.assertEqual(3, receipt["message_count"])
            self.assertEqual(1, receipt["candidate_reply_count"])
            self.assertEqual("graded", state["status"])
            self.assertTrue(state["steps"]["graded"])
            self.assertEqual(receipt["received_reply_event_id"], state["idempotency"]["processed_reply_event_ids"][0])
            self.assertEqual(10, grading_report["total_questions"])

            aicx_study_fixture.command_daily_grade_poll(poll_args)
            duplicate_receipt = aicx_study_fixture.load_json(out_dir / "slack_thread_poll_receipt.json")
            duplicate_state = aicx_study_fixture.load_json(out_dir / "run_state.json")

            self.assertEqual("duplicate_reply_skipped", duplicate_receipt["status"])
            self.assertTrue(duplicate_state["idempotency"]["duplicate_grading_skipped"])
            self.assertEqual("duplicate_reply_skipped", duplicate_state["last_event"]["status"])

    def test_daily_maintain_polls_sent_run_and_records_no_reply(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            out_root = Path(tmp_dir)
            out_dir = out_root / "2026-06-21"
            dispatch_args = type(
                "Args",
                (),
                {
                    "study_schedule": FIXTURE_DIR / "study_schedule.json",
                    "topic_map": FIXTURE_DIR / "topic_map.json",
                    "question_bank": FIXTURE_DIR / "question_bank.fixture.json",
                    "adaptive_plan": None,
                    "date": "2026-06-21",
                    "question_count": None,
                    "out_dir": out_dir,
                    "run_state": None,
                    "mode": "dry-run",
                    "preview_count": 3,
                    "env_file": None,
                    "now": "2026-06-21T04:30:00+09:00",
                    "force": False,
                },
            )()
            aicx_study_fixture.command_daily_dispatch(dispatch_args)
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            state["slack"]["channel_id"] = "C_FIXTURE"
            state["slack"]["thread_ts"] = "1782074738.865009"
            aicx_study_fixture.write_json(out_dir / "run_state.json", state)
            messages_fixture_path = out_dir / "thread_messages.json"
            aicx_study_fixture.write_json(
                messages_fixture_path,
                {
                    "messages": [
                        {
                            "type": "message",
                            "user": "U_PARENT",
                            "text": "今日の問題",
                            "ts": "1782074738.865009",
                        }
                    ],
                },
            )
            args = self.make_daily_maintain_args(
                out_root,
                "2026-06-21T04:45:00+09:00",
                thread_messages_fixture=messages_fixture_path,
            )

            aicx_study_fixture.command_daily_maintain(args)
            receipt = aicx_study_fixture.load_json(out_dir / "maintenance_receipt.json")
            poll_receipt = aicx_study_fixture.load_json(out_dir / "slack_thread_poll_receipt.json")
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")

            self.assertEqual("no_reply_found", receipt["status"])
            self.assertEqual("thread_poll", receipt["action"])
            self.assertEqual("no_reply_found", poll_receipt["status"])
            self.assertEqual("dispatched", state["status"])
            self.assertIsNone(state["failure"])
            self.assertFalse(state["steps"]["graded"])

    def test_daily_status_reports_waiting_for_reply_and_next_action(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            out_dir = Path(tmp_dir)
            dispatch_args = type(
                "Args",
                (),
                {
                    "study_schedule": FIXTURE_DIR / "study_schedule.json",
                    "topic_map": FIXTURE_DIR / "topic_map.json",
                    "question_bank": FIXTURE_DIR / "question_bank.fixture.json",
                    "adaptive_plan": None,
                    "date": "2026-06-21",
                    "question_count": None,
                    "out_dir": out_dir,
                    "run_state": None,
                    "mode": "dry-run",
                    "preview_count": 3,
                    "env_file": None,
                    "now": "2026-06-21T04:30:00+09:00",
                    "force": False,
                },
            )()
            aicx_study_fixture.command_daily_dispatch(dispatch_args)
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            state["status"] = "failed"
            state["slack"]["channel_id"] = "C_FIXTURE"
            state["slack"]["thread_ts"] = "1782074738.865009"
            state["last_event"] = {
                "event_type": "thread_poll",
                "status": "no_reply_found",
                "at_unix_seconds": 1782074700,
            }
            aicx_study_fixture.write_json(out_dir / "run_state.json", state)
            args = type("Args", (), {"run_state": out_dir / "run_state.json"})()

            output = io.StringIO()
            with redirect_stdout(output):
                aicx_study_fixture.command_daily_status(args)
            rendered = output.getvalue()

            self.assertIn("AICX Study Bot - 2026-06-21", rendered)
            self.assertIn("Status:\n  waiting_for_reply", rendered)
            self.assertIn("raw_run_state: failed", rendered)
            self.assertIn("thread_ts: 1782074738.865009", rendered)
            self.assertIn("Next Action:", rendered)
            self.assertIn("daily-maintain", rendered)
            self.assertIn("quiz_prompt_md:", rendered)

    def test_daily_status_reports_graded_noop(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            out_root = Path(tmp_dir)
            out_dir = out_root / "2026-06-21"
            dispatch_args = type(
                "Args",
                (),
                {
                    "study_schedule": FIXTURE_DIR / "study_schedule.json",
                    "topic_map": FIXTURE_DIR / "topic_map.json",
                    "question_bank": FIXTURE_DIR / "question_bank.fixture.json",
                    "adaptive_plan": None,
                    "date": "2026-06-21",
                    "question_count": None,
                    "out_dir": out_dir,
                    "run_state": None,
                    "mode": "dry-run",
                    "preview_count": 3,
                    "env_file": None,
                    "now": "2026-06-21T04:30:00+09:00",
                    "force": False,
                },
            )()
            aicx_study_fixture.command_daily_dispatch(dispatch_args)
            quiz_set = aicx_study_fixture.load_json(out_dir / "quiz_set.json")
            reply_event = aicx_study_fixture.build_slack_reply_event_fixture(
                quiz_set,
                aicx_study_fixture.generate_fixture_submission(quiz_set),
            )
            reply_path = out_dir / "slack_reply_event.json"
            aicx_study_fixture.write_json(reply_path, reply_event)
            grade_args = type(
                "Args",
                (),
                {
                    "run_state": out_dir / "run_state.json",
                    "quiz_set": out_dir / "quiz_set.json",
                    "slack_reply_event": reply_path,
                    "out_dir": out_dir,
                    "mode": "dry-run",
                    "env_file": None,
                    "now": "2026-06-21T04:40:00+09:00",
                },
            )()
            aicx_study_fixture.command_daily_grade(grade_args)
            args = type("Args", (), {"run_state": out_dir / "run_state.json"})()

            output = io.StringIO()
            with redirect_stdout(output):
                aicx_study_fixture.command_daily_status(args)
            rendered = output.getvalue()

            self.assertIn("Status:\n  graded", rendered)
            self.assertIn("今日分は完了", rendered)
            self.assertIn("grading_report:", rendered)

    def test_daily_maintain_grades_reply_once_then_noops(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            out_root = Path(tmp_dir)
            out_dir = out_root / "2026-06-21"
            dispatch_args = type(
                "Args",
                (),
                {
                    "study_schedule": FIXTURE_DIR / "study_schedule.json",
                    "topic_map": FIXTURE_DIR / "topic_map.json",
                    "question_bank": FIXTURE_DIR / "question_bank.fixture.json",
                    "adaptive_plan": None,
                    "date": "2026-06-21",
                    "question_count": None,
                    "out_dir": out_dir,
                    "run_state": None,
                    "mode": "dry-run",
                    "preview_count": 3,
                    "env_file": None,
                    "now": "2026-06-21T04:30:00+09:00",
                    "force": False,
                },
            )()
            aicx_study_fixture.command_daily_dispatch(dispatch_args)
            quiz_set = aicx_study_fixture.load_json(out_dir / "quiz_set.json")
            submission = aicx_study_fixture.generate_fixture_submission(quiz_set)
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            state["slack"]["channel_id"] = "C_FIXTURE"
            state["slack"]["thread_ts"] = "1782074738.865009"
            aicx_study_fixture.write_json(out_dir / "run_state.json", state)
            messages_fixture_path = out_dir / "thread_messages.json"
            aicx_study_fixture.write_json(
                messages_fixture_path,
                {
                    "messages": [
                        {
                            "type": "message",
                            "user": "U_FIXTURE",
                            "text": aicx_study_fixture.render_answer_submission_text(submission),
                            "ts": "1782074800.000000",
                            "thread_ts": "1782074738.865009",
                        }
                    ],
                },
            )
            args = self.make_daily_maintain_args(
                out_root,
                "2026-06-21T04:45:00+09:00",
                thread_messages_fixture=messages_fixture_path,
            )

            aicx_study_fixture.command_daily_maintain(args)
            receipt = aicx_study_fixture.load_json(out_dir / "maintenance_receipt.json")
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")

            self.assertEqual("reply_found_graded_and_sent", receipt["status"])
            self.assertEqual("thread_poll", receipt["action"])
            self.assertEqual("graded", state["status"])
            self.assertEqual(1, len(state["idempotency"]["processed_reply_event_ids"]))

            aicx_study_fixture.command_daily_maintain(args)
            duplicate_receipt = aicx_study_fixture.load_json(out_dir / "maintenance_receipt.json")
            duplicate_state = aicx_study_fixture.load_json(out_dir / "run_state.json")

            self.assertEqual("already_graded", duplicate_receipt["status"])
            self.assertEqual("noop", duplicate_receipt["action"])
            self.assertEqual(1, len(duplicate_state["idempotency"]["processed_reply_event_ids"]))

    def test_daily_grade_poll_records_no_reply_without_failure(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            out_dir = Path(tmp_dir)
            dispatch_args = type(
                "Args",
                (),
                {
                    "study_schedule": FIXTURE_DIR / "study_schedule.json",
                    "topic_map": FIXTURE_DIR / "topic_map.json",
                    "question_bank": FIXTURE_DIR / "question_bank.fixture.json",
                    "adaptive_plan": None,
                    "date": "2026-06-21",
                    "question_count": None,
                    "out_dir": out_dir,
                    "run_state": None,
                    "mode": "dry-run",
                    "preview_count": 3,
                    "env_file": None,
                    "now": "2026-06-21T04:30:00+09:00",
                    "force": False,
                },
            )()
            aicx_study_fixture.command_daily_dispatch(dispatch_args)
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            state["slack"]["channel_id"] = "C_FIXTURE"
            state["slack"]["thread_ts"] = "1782074738.865009"
            aicx_study_fixture.write_json(out_dir / "run_state.json", state)
            messages_fixture_path = out_dir / "thread_messages.json"
            aicx_study_fixture.write_json(
                messages_fixture_path,
                {
                    "messages": [
                        {
                            "type": "message",
                            "user": "U_PARENT",
                            "text": "今日の問題",
                            "ts": "1782074738.865009",
                        }
                    ],
                },
            )

            poll_args = type(
                "Args",
                (),
                {
                    "run_state": out_dir / "run_state.json",
                    "quiz_set": None,
                    "out_dir": out_dir,
                    "mode": "dry-run",
                    "thread_messages_fixture": messages_fixture_path,
                    "env_file": None,
                    "now": "2026-06-21T04:45:00+09:00",
                },
            )()
            aicx_study_fixture.command_daily_grade_poll(poll_args)

            receipt = aicx_study_fixture.load_json(out_dir / "slack_thread_poll_receipt.json")
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")

            self.assertEqual("no_reply_found", receipt["status"])
            self.assertEqual("dispatched", state["status"])
            self.assertEqual("no_reply_found", state["last_event"]["status"])
            self.assertFalse(state["steps"]["graded"])

    def test_daily_grade_poll_posts_invalid_reply_error_response(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            out_dir = Path(tmp_dir)
            dispatch_args = type(
                "Args",
                (),
                {
                    "study_schedule": FIXTURE_DIR / "study_schedule.json",
                    "topic_map": FIXTURE_DIR / "topic_map.json",
                    "question_bank": FIXTURE_DIR / "question_bank.fixture.json",
                    "adaptive_plan": None,
                    "date": "2026-06-21",
                    "question_count": None,
                    "out_dir": out_dir,
                    "run_state": None,
                    "mode": "dry-run",
                    "preview_count": 3,
                    "env_file": None,
                    "now": "2026-06-21T04:30:00+09:00",
                    "force": False,
                },
            )()
            aicx_study_fixture.command_daily_dispatch(dispatch_args)
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")
            state["slack"]["channel_id"] = "C_FIXTURE"
            state["slack"]["thread_ts"] = "1782074738.865009"
            aicx_study_fixture.write_json(out_dir / "run_state.json", state)
            messages_fixture_path = out_dir / "thread_messages.json"
            aicx_study_fixture.write_json(
                messages_fixture_path,
                {
                    "messages": [
                        {
                            "type": "message",
                            "user": "U_FIXTURE",
                            "text": "1:A",
                            "ts": "1782074800.000000",
                            "thread_ts": "1782074738.865009",
                        }
                    ],
                },
            )
            poll_args = type(
                "Args",
                (),
                {
                    "run_state": out_dir / "run_state.json",
                    "quiz_set": None,
                    "out_dir": out_dir,
                    "mode": "dry-run",
                    "thread_messages_fixture": messages_fixture_path,
                    "env_file": None,
                    "now": "2026-06-21T04:45:00+09:00",
                },
            )()

            aicx_study_fixture.command_daily_grade_poll(poll_args)
            receipt = aicx_study_fixture.load_json(out_dir / "slack_thread_poll_receipt.json")
            delivery_receipt = aicx_study_fixture.load_json(out_dir / "slack_grading_delivery_receipt.json")
            state = aicx_study_fixture.load_json(out_dir / "run_state.json")

            self.assertEqual("invalid_reply_found", receipt["status"])
            self.assertEqual("invalid_reply_error", delivery_receipt["response_kind"])
            self.assertEqual("invalid_reply", state["status"])
            self.assertIn("missing=", state["failure"]["errors"][0])

    def test_adaptive_plan_marks_weak_topics_and_changes_next_question_mix(self):
        grading_report = aicx_study_fixture.load_json(FIXTURE_DIR / "grading_report.json")
        adaptive_plan = aicx_study_fixture.build_adaptive_plan(
            self.schedule,
            self.topic_map,
            self.question_bank,
            [grading_report],
            "2026-06-22",
        )

        self.assertEqual("aicx.adaptive_plan.v0", adaptive_plan["schema_version"])
        self.assertEqual("ADAPTIVE-ASB-2026-06-22", adaptive_plan["adaptive_plan_id"])
        self.assertEqual(10, adaptive_plan["question_count"])
        self.assertFalse(adaptive_plan["selection_policy"]["pdf_ingest_used"])
        self.assertFalse(adaptive_plan["selection_policy"]["llm_generation_used"])
        weak_topic_ids = {topic["topic_id"] for topic in adaptive_plan["weak_topics"]}
        self.assertEqual({"TOPIC-5D", "TOPIC-ORG-KPI"}, weak_topic_ids)
        allocations = {
            item["topic_id"]: item["question_count"]
            for item in adaptive_plan["topic_allocations"]
        }
        self.assertEqual(10, sum(allocations.values()))
        self.assertEqual(4, allocations["TOPIC-5D"])
        self.assertEqual(3, allocations["TOPIC-ORG-KPI"])
        self.assertEqual(1, allocations["TOPIC-BRANCHING-CONTEXT"])
        self.assertEqual(1, allocations["TOPIC-WORKFLOW-CHECK"])
        self.assertEqual(1, allocations["TOPIC-ADOPTION-GOVERNANCE"])

    def test_adaptive_plan_selects_question_bank_first_then_topic_map_fallback(self):
        grading_report = aicx_study_fixture.load_json(FIXTURE_DIR / "grading_report.json")
        adaptive_plan = aicx_study_fixture.build_adaptive_plan(
            self.schedule,
            self.topic_map,
            self.question_bank,
            [grading_report],
            "2026-06-22",
        )
        selection_by_topic = {}
        for item in adaptive_plan["question_selection"]:
            selection_by_topic.setdefault(item["topic_id"], []).append(item)

        self.assertEqual(10, len(adaptive_plan["question_selection"]))
        five_d_sources = [item["source"] for item in selection_by_topic["TOPIC-5D"]]
        org_kpi_sources = [item["source"] for item in selection_by_topic["TOPIC-ORG-KPI"]]
        self.assertEqual(
            ["question_bank_fixture", "question_bank_fixture", "manual_fixture_topic_map", "manual_fixture_topic_map"],
            five_d_sources,
        )
        self.assertEqual(
            ["question_bank_fixture", "question_bank_fixture", "manual_fixture_topic_map"],
            org_kpi_sources,
        )
        self.assertEqual(
            "question_bank_priority",
            selection_by_topic["TOPIC-BRANCHING-CONTEXT"][0]["selection_reason"],
        )

    def test_unknown_date_fails_closed(self):
        with self.assertRaises(ValueError):
            aicx_study_fixture.generate_quiz_set(
                self.schedule,
                self.topic_map,
                "2026-08-01",
                30,
            )


if __name__ == "__main__":
    unittest.main()
