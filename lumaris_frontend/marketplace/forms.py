# pylint: disable=too-few-public-methods
"""
This module defines Django forms for the marketplace app.
Classes:
    JobSubmissionForm: A ModelForm for submitting computational jobs,
    allowing users to specify job title, description, required CPU cores,
    memory, expected duration, and priority. Custom widgets and labels are
    provided for enhanced form rendering and user experience.
"""
from django import forms

from marketplace.models import Job


class JobSubmissionForm(forms.ModelForm):
    """A ModelForm for submitting computational jobs,"""

    class Meta:
        """Meta class for JobSubmissionForm."""

        model = Job
        fields = [
            "title",
            "description",
            "cpu_cores_required",
            "memory_mb_required",
            "expected_duration_sec",
            "priority",
        ]
        widgets = {
            "title": forms.TextInput(
                attrs={"class": "form-control", "placeholder": "Job Title"}
            ),
            "description": forms.Textarea(
                attrs={
                    "class": "form-control",
                    "rows": 4,
                    "placeholder": "Describe your computational task...",
                }
            ),
            "cpu_cores_required": forms.NumberInput(
                attrs={"class": "form-control", "min": 1}
            ),
            "memory_mb_required": forms.NumberInput(
                attrs={"class": "form-control", "min": 512}
            ),
            "expected_duration_sec": forms.NumberInput(
                attrs={"class": "form-control", "min": 60}
            ),
            "priority": forms.Select(attrs={"class": "form-control"}),
        }
        labels = {
            "cpu_cores_required": "CPU Cores Required",
            "memory_mb_required": "Memory Required (MB)",
            "expected_duration_sec": "Expected Duration (seconds)",
        }
        help_texts = {
            "title": "Enter a brief title for your job.",
            "description": "Provide a detailed description of the job.",
            "cpu_cores_required": "Specify the number of CPU cores required for the job.",
            "memory_mb_required": "Specify the amount of memory required (in MB).",
            "expected_duration_sec": "Estimate the expected duration of the job (in seconds).",
            "priority": "Select the priority level for this job.",
        }
        error_messages = {
            "title": {
                "required": "Please enter a title for your job.",
            },
            "description": {
                "required": "Please provide a description of the job.",
            },
            "cpu_cores_required": {
                "required": "Please specify the number of CPU cores required.",
                "invalid": "Please enter a valid number of CPU cores.",
            },
            "memory_mb_required": {
                "required": "Please specify the amount of memory required.",
                "invalid": "Please enter a valid amount of memory (in MB).",
            },
            "expected_duration_sec": {
                "required": "Please estimate the expected duration of the job.",
                "invalid": "Please enter a valid duration (in seconds).",
            },
        }
