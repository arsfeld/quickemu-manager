#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "click>=8.1",
#     "rich>=13.0",
#     "gitpython>=3.1",
# ]
# ///

"""
Release script for QuickEmu Manager
Creates GitHub releases with LLM-generated release notes

Release Command Flow:
┌─────────────┐                                                         
│ Start       │                                                         
└──────┬──────┘                                                         
       │                                                                 
       ▼                                                                 
┌─────────────┐     YES    ┌─────────────────┐                        
│ Version     ├────────────▶│ Tag exists?     │                        
│ specified?  │             └────┬────────────┘                        
└──────┬──────┘                  │       │                             
       │ NO                      │ YES   │ NO                          
       ▼                         ▼       └────────┐                    
┌─────────────┐         ┌─────────────────┐      │                    
│ New commits │         │ Regenerate notes│      │                    
│ since last  │         │ for existing    │      │                    
│ release?    │         │ release         │      │                    
└──┬───────┬──┘         └────────┬────────┘      │                    
   │       │                     │                │                    
   │ YES   │ NO                  ▼                ▼                    
   │       └─────────▶  ┌─────────────────┐  ┌────────────────┐       
   │                    │ Update existing │  │ Create new     │       
   │                    │ release with    │  │ release with   │       
   ▼                    │ gh release edit │  │ specified      │       
┌─────────────┐         └─────────────────┘  │ version        │       
│ LLM suggests│                              └───────┬────────┘       
│ new version │                                      │                 
└──────┬──────┘                                      │                 
       │                                              │                 
       └──────────────────────┬───────────────────────┘                
                              ▼                                         
                      ┌─────────────┐                                   
                      │ Create new  │                                   
                      │ release with│                                   
                      │ gh release  │                                   
                      │ create      │                                   
                      └─────────────┘
"""

import subprocess
import sys
import tempfile
import re
from pathlib import Path
from typing import Optional, Tuple
import click
from rich.console import Console
from rich.panel import Panel
from rich.prompt import Confirm
from git import Repo

console = Console()

RELEASE_NOTES_INSTRUCTIONS = """Please create professional release notes that:
- Start with a brief summary of the release
- Group changes by category (Features, Bug Fixes, Improvements, etc.)
- Highlight breaking changes if any
- Thank contributors if there are any
- Keep it concise but informative
- Use markdown formatting

The project has two main components:
- QuickEmu Manager: A VM management GUI (GTK4 and Dioxus versions)
- SPICE Client: A Rust implementation of the SPICE protocol

IMPORTANT: Output ONLY the release notes text. Do not create any files or use any tools."""


def run_command(cmd: list[str], capture=True) -> Tuple[int, str]:
    """Run a command and return exit code and output."""
    try:
        if capture:
            result = subprocess.run(cmd, capture_output=True, text=True, check=True)
            return 0, result.stdout.strip()
        else:
            result = subprocess.run(cmd, check=True)
            return 0, ""
    except subprocess.CalledProcessError as e:
        return e.returncode, e.stdout if capture else ""


def validate_version(version: str) -> bool:
    """Validate version format."""
    return bool(re.match(r'^v\d+\.\d+\.\d+$', version))


def tag_exists(tag: str) -> bool:
    """Check if a git tag exists."""
    try:
        repo = Repo(".")
        return tag in [t.name for t in repo.tags]
    except:
        return False


def get_last_tag() -> Optional[str]:
    """Get the most recent tag."""
    try:
        repo = Repo(".")
        tags = sorted(repo.tags, key=lambda t: t.commit.committed_datetime)
        return tags[-1].name if tags else None
    except:
        return None


def get_commits_since_tag(tag: Optional[str]) -> list[str]:
    """Get commits since the specified tag."""
    try:
        repo = Repo(".")
        if tag:
            commits = list(repo.iter_commits(f'{tag}..HEAD'))
        else:
            commits = list(repo.iter_commits('HEAD'))
        
        return [f"{c.hexsha[:7]} {c.summary}" for c in commits]
    except:
        return []


def call_llm(prompt: str, llm: str) -> Optional[str]:
    """Call the LLM with the given prompt."""
    # Check if LLM CLI exists
    if llm == "claude":
        ret, _ = run_command(["which", "claude"])
        if ret != 0:
            console.print("[red]Error: claude CLI not found. Install from: https://github.com/anthropics/claude-code[/red]")
            return None
    elif llm == "gemini":
        ret, _ = run_command(["which", "gemini"])
        if ret != 0:
            console.print("[red]Error: gemini CLI not found. Install from: https://github.com/google-gemini/gemini-cli[/red]")
            return None
    else:
        console.print(f"[red]Error: Unknown LLM '{llm}'. Use 'claude' or 'gemini'[/red]")
        return None

    try:
        # Call LLM with prompt via stdin
        process = subprocess.Popen(
            [llm], 
            stdin=subprocess.PIPE, 
            stdout=subprocess.PIPE, 
            stderr=subprocess.PIPE,
            text=True
        )
        output, error = process.communicate(input=prompt)
        
        if process.returncode == 0:
            return output.strip()
        else:
            console.print(f"[red]Error calling {llm}: {error}[/red]")
            return None
    except Exception as e:
        console.print(f"[red]Error calling {llm}: {str(e)}[/red]")
        return None


def create_release_prompt(version: Optional[str], last_version: str, commits: list[str], regenerate: bool) -> str:
    """Create the prompt for the LLM."""
    if regenerate:
        return f"""Generate release notes for the existing version {last_version} of QuickEmu Manager.

This is a regeneration of release notes for an already-released version.
Please analyze the git history up to this tag and create release notes.

{RELEASE_NOTES_INSTRUCTIONS}"""
    
    elif version:
        # Specified version
        commits_text = '\n'.join(commits)
        return f"""Generate release notes for version {version} of QuickEmu Manager.

Previous version: {last_version}
New version: {version}

Commits since last release:
{commits_text}

{RELEASE_NOTES_INSTRUCTIONS}"""
    
    else:
        # Auto-suggest version
        commits_text = '\n'.join(commits)
        return f"""Analyze the following commits and suggest a new version number for QuickEmu Manager.

Current version: {last_version}

Commits since last release:
{commits_text}

Based on semantic versioning (semver):
- MAJOR version (X.0.0): incompatible API changes, breaking changes
- MINOR version (0.X.0): new functionality in a backwards compatible manner
- PATCH version (0.0.X): backwards compatible bug fixes

Please respond with:
1. First line: Just the suggested version number (e.g., v1.2.3)
2. Second line: Brief explanation of why this version was chosen
3. Rest: Professional release notes

{RELEASE_NOTES_INSTRUCTIONS}"""


@click.command()
@click.argument('version', required=False, default='')
@click.option('--llm', default='claude', help='LLM to use (claude or gemini)')
def main(version: str, llm: str):
    """Create a new release with LLM-generated release notes."""
    
    # Initialize git repo
    try:
        repo = Repo(".")
    except:
        console.print("[red]Error: Not in a git repository[/red]")
        sys.exit(1)

    # Check if version was provided
    if version:
        if not validate_version(version):
            console.print("[red]Error: Version must be in format vX.Y.Z (e.g., v1.0.0)[/red]")
            sys.exit(1)
        
        if tag_exists(version):
            console.print(f"[yellow]Tag {version} already exists. Generating release notes for it...[/yellow]")
            regenerate_only = True
            specified_version = version
            last_version = version
            commits = []
        else:
            console.print(f"[green]Tag {version} does not exist. Will create new release with this version...[/green]")
            regenerate_only = False
            use_specified_version = True
            specified_version = version
            last_tag = get_last_tag()
            last_version = last_tag or "v0.0.0"
            console.print(f"Comparing changes from {last_version} to HEAD...")
            commits = get_commits_since_tag(last_tag)
    else:
        use_specified_version = False
        last_tag = get_last_tag()
        
        if not last_tag:
            console.print("[yellow]No previous release found. This will be the first release...[/yellow]")
            commits = get_commits_since_tag(None)
            last_version = "v0.0.0"
        else:
            console.print(f"Comparing changes from {last_tag} to HEAD...")
            commits = get_commits_since_tag(last_tag)
            last_version = last_tag
        
        if not commits:
            console.print(f"[yellow]No new commits since last release ({last_tag})[/yellow]")
            console.print("[yellow]Generating release notes for existing version...[/yellow]")
            regenerate_only = True
        else:
            regenerate_only = False
            console.print(f"[green]Found {len(commits)} commits since last release[/green]")

    # Create prompt and call LLM
    prompt = create_release_prompt(
        version if use_specified_version else None,
        last_version,
        commits,
        regenerate_only
    )
    
    console.print(f"\n[blue]Analyzing changes with {llm}...[/blue]")
    llm_output = call_llm(prompt, llm)
    
    if not llm_output:
        console.print("[red]Failed to generate release notes[/red]")
        sys.exit(1)

    if regenerate_only:
        # For regeneration, update the existing release
        release_notes = llm_output
        
        console.print(Panel(release_notes, title=f"Generated Release Notes for {last_version}", border_style="green"))
        
        if not Confirm.ask(f"Update release {last_version} with these notes?"):
            console.print("[yellow]Update cancelled.[/yellow]")
            sys.exit(0)
        
        console.print(f"[blue]Updating release {last_version}...[/blue]")
        ret, _ = run_command(["gh", "release", "edit", last_version, "--notes", release_notes], capture=False)
        
        if ret == 0:
            console.print(f"[green]✅ Release {last_version} updated successfully![/green]")
            ret, url = run_command(["gh", "release", "view", last_version, "--json", "url", "-q", ".url"])
            if ret == 0:
                console.print(f"[blue]Visit the release page: {url}[/blue]")
        else:
            console.print("[red]Failed to update release[/red]")
            sys.exit(1)
    else:
        # Handle new release creation
        if use_specified_version:
            new_version = specified_version
            release_notes = llm_output
        else:
            # Parse LLM output
            lines = llm_output.strip().split('\n')
            if len(lines) < 3:
                console.print("[red]Error: Invalid LLM output format[/red]")
                console.print(llm_output)
                sys.exit(1)
            
            suggested_version = lines[0].strip()
            version_explanation = lines[1].strip()
            release_notes = '\n'.join(lines[2:]).strip()
            
            if not validate_version(suggested_version):
                console.print(f"[red]Error: LLM suggested invalid version format: {suggested_version}[/red]")
                console.print("[yellow]Full LLM output:[/yellow]")
                console.print(llm_output)
                sys.exit(1)
            
            if tag_exists(suggested_version):
                console.print(f"[red]Error: Tag {suggested_version} already exists[/red]")
                sys.exit(1)
            
            new_version = suggested_version
        
        # Show the analysis
        console.print("\n[bold]Version Analysis[/bold]")
        console.print(f"Current version: {last_version}")
        console.print(f"New version: {new_version}")
        if not use_specified_version and 'version_explanation' in locals():
            console.print(f"Reason: {version_explanation}")
        
        console.print(Panel(release_notes, title="Generated Release Notes", border_style="green"))
        
        if not Confirm.ask(f"Create release {new_version} with these notes?"):
            console.print("[yellow]Release cancelled.[/yellow]")
            sys.exit(0)
        
        # Create and push the tag
        console.print(f"[blue]Creating and pushing tag {new_version}...[/blue]")
        ret, _ = run_command(["git", "tag", "-a", new_version, "-m", f"Release {new_version}"], capture=False)
        if ret != 0:
            console.print("[red]Failed to create tag[/red]")
            sys.exit(1)
            
        ret, _ = run_command(["git", "push", "origin", new_version], capture=False)
        if ret != 0:
            console.print("[red]Failed to push tag[/red]")
            sys.exit(1)
        
        # Create GitHub release
        console.print("[blue]Creating GitHub release...[/blue]")
        ret, _ = run_command(["gh", "release", "create", new_version, "--title", new_version, "--notes", release_notes, "--draft"], capture=False)
        
        if ret == 0:
            console.print("[green]✅ Draft release created successfully![/green]")
            ret, url = run_command(["gh", "release", "view", new_version, "--json", "url", "-q", ".url"])
            if ret == 0:
                console.print(f"[blue]Visit the release page to review and publish: {url}[/blue]")
        else:
            console.print("[red]Failed to create release[/red]")
            sys.exit(1)


if __name__ == "__main__":
    main()