import os
import zipfile
import shutil
import gitlab

GITLAB_URL = "https://gitlab.gnome.org"
PROJECT_ID = 665  # GNOME/GTK project
JOB_NAME = "macos: [macosarm]"
ARTIFACT_NAME = "artifacts.zip"
SAVE_PATH = "./"
TARGET_FILE = "_build/gtk/GdkMacos-4.0.gir"

PERSONAL_ACCESS_TOKEN = os.environ.get("GITLAB_KEY")
if not PERSONAL_ACCESS_TOKEN:
    raise RuntimeError("GITLAB_KEY environment variable is not set")


def download_and_extract_artifact(job, target_file, destination_path):
    artifact_path = os.path.join(SAVE_PATH, ARTIFACT_NAME)

    # Download artifacts archive
    with open(artifact_path, "wb") as f:
        job.artifacts(streamed=True, action=f.write)

    print(f"Artifact saved to: {artifact_path}")

    try:
        temp_dir = os.path.join(os.path.dirname(artifact_path), "temp_extracted")
        os.makedirs(temp_dir, exist_ok=True)

        with zipfile.ZipFile(artifact_path, "r") as zip_ref:
            zip_ref.extractall(temp_dir)

        target_file_path = os.path.join(temp_dir, target_file)
        if os.path.exists(target_file_path):
            shutil.copy(target_file_path, destination_path)
            print(f"Copied '{target_file_path}' to '{destination_path}'.")
        else:
            print(f"File '{target_file}' not found in the extracted contents.")

    finally:
        shutil.rmtree(temp_dir, ignore_errors=True)
        if os.path.exists(artifact_path):
            os.remove(artifact_path)
        print("Temporary files cleaned up.")


def main():
    gl = gitlab.Gitlab(
        GITLAB_URL,
        private_token=PERSONAL_ACCESS_TOKEN,
    )

    project = gl.projects.get(PROJECT_ID)

    pipelines = project.pipelines.list(
        order_by="updated_at",
        sort="desc",
        per_page=20,
    )

    if not pipelines:
        raise RuntimeError("No pipelines found")

    for pipeline in pipelines:
        pipeline = project.pipelines.get(pipeline.id)

        jobs = pipeline.jobs.list(all=True)

        for job in jobs:
            if job.name != JOB_NAME:
                continue
            if job.status != "success":
                continue
            if not job.artifacts_file:
                continue

            print(f"Job '{job.name}' succeeded (Job ID: {job.id}).")

            download_and_extract_artifact(
                job,
                TARGET_FILE,
                "./",
            )
            return

    print("No successful job with artifacts found.")


if __name__ == "__main__":
    main()
