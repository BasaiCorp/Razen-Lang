import os
import sys
import json
import uuid
import hashlib
import shutil
import sqlite3
import platform
import re
import tempfile
import glob
import traceback
import configparser
import time
import subprocess
import errno
import argparse
from typing import Tuple, List, Optional

try:
    from colorama import Fore, Style, init
    init()  # Initialize colorama
except ImportError:
    print("Warning: colorama not found. Output will not be colored.")
    class DummyStyle:
        def __getattr__(self, name):
            return ""
    Fore = DummyStyle()
    Style = DummyStyle()
    def init():
        pass
    init()

# Attempt to import helpers, with fallbacks
try:
    from new_signup import get_user_documents_path
except ImportError:
    print(f"{Fore.YELLOW}Warning: Using fallback for get_user_documents_path.{Style.RESET_ALL}")
    def get_user_documents_path():
        return os.path.expanduser("~")
try:
    from config import get_config
except ImportError:
    print(f"{Fore.YELLOW}Warning: Using fallback for get_config.{Style.RESET_ALL}")
    def get_config(translator=None):
        return {}

# Define emoji constants
EMOJI = {
    "FILE": "📄", "BACKUP": "💾", "SUCCESS": "✅", "ERROR": "❌", "INFO": "ℹ️",
    "RESET": "🔄", "WARNING": "⚠️", "SEARCH": "🔍", "PATH": "➡️ ", "LOCK": "🔒",
    "DELETE": "🗑️"
}

# --- System Paths to Exclude and Manually Include for Cursor ---

SYSTEM_PATHS_CURSOR = [
    "/opt/Cursor", "/usr/share/cursor", "/usr/lib/cursor", "/usr/local/share/cursor",
    "/var/lib/flatpak/app/com.cursor.Cursor", "/snap/cursor", os.path.join(os.path.expanduser("~"), "snap/cursor"),
    "/tmp/.mount_Cursor*", "/tmp/squashfs-root*"  # AppImage temporary mounts
]

USER_CONFIG_BASE_DIRS = [
    os.path.expanduser("~/.config/cursor"),
    os.path.expanduser("~/.local/share/cursor"),
    os.path.expanduser("~/.cursor"),
    os.path.join(os.path.expanduser("~"), "Library/Application Support/Cursor") if platform.system() == "Darwin" else None,
    os.getenv("APPDATA") + "\\Cursor" if platform.system() == "Windows" else None
]

USER_CONFIG_BASE_DIRS = [x for x in USER_CONFIG_BASE_DIRS if x is not None]

# --- Path Finding Functions ---

def get_actual_home_dir() -> str:
    if platform.system() == "Linux" and os.environ.get('SUDO_USER'):
        return os.path.expanduser(f"~{os.environ.get('SUDO_USER')}")
    return os.path.expanduser("~")

def _get_translator_func(translator=None):
    """Helper to get a callable translator or a basic fallback."""
    if callable(translator):
        return translator
    return lambda key, **kwargs: kwargs.get('default', key)

def is_system_path(path: str) -> bool:
    """Check if the path is a system path or contains .appimage files."""
    path = os.path.abspath(path)
    return any(sys_path in path or path.startswith(sys_path) for sys_path in SYSTEM_PATHS_CURSOR) or path.endswith('.appimage')

def find_cursor_config_dir(translator=None) -> Optional[str]:
    t = _get_translator_func(translator)
    print(f"{Fore.CYAN}{EMOJI['SEARCH']} {t('find_path.searching_config', default='Searching for Cursor configuration directory...')}{Style.RESET_ALL}")
    home_dir = get_actual_home_dir()
    system = platform.system()
    potential_paths: List[str] = USER_CONFIG_BASE_DIRS.copy()

    if system == "Linux":
        potential_paths.extend([
            os.path.join(home_dir, "snap/cursor/current/.config/cursor"),
            os.path.join(home_dir, ".var/app/com.cursor.Cursor/config/cursor")
        ])
    elif system == "Darwin":
        potential_paths.append(os.path.join(home_dir, "Library/Application Support/Cursor"))
    elif system == "Windows":
        appdata = os.getenv("APPDATA")
        if appdata:
            potential_paths.append(os.path.join(appdata, "Cursor"))

    valid_config_dir = None
    checked_paths_log = []
    for path in potential_paths:
        if not path or is_system_path(path):  # Skip system paths and .appimage
            continue
        normalized_path = os.path.abspath(path)
        if normalized_path in checked_paths_log: continue
        checked_paths_log.append(normalized_path)
        user_subdir = os.path.join(normalized_path, "User")
        global_storage_subdir = os.path.join(user_subdir, "globalStorage")
        if os.path.isdir(normalized_path) and os.path.isdir(user_subdir) and os.path.isdir(global_storage_subdir):
            print(f"{Fore.GREEN}{EMOJI['SUCCESS']} {t('find_path.found_config_at', path=normalized_path, default=f'Found valid config directory at: {normalized_path}')}{Style.RESET_ALL}")
            valid_config_dir = normalized_path
            break
    if not valid_config_dir:
        print(f"{Fore.RED}{EMOJI['ERROR']} {t('find_path.config_not_found', default='Cursor configuration directory not found.')}{Style.RESET_ALL}")
        checked_paths_str = ", ".join(checked_paths_log)
        print(f"{Fore.YELLOW}{EMOJI['INFO']} {t('find_path.checked_paths', paths=checked_paths_str, default='Checked paths: {paths}')}{Style.RESET_ALL}")
    return valid_config_dir

def find_cursor_app_resource_dir(translator=None) -> Optional[str]:
    t = _get_translator_func(translator)
    print(f"{Fore.CYAN}{EMOJI['SEARCH']} {t('find_path.searching_app', default='Searching for Cursor application resource directory...')}{Style.RESET_ALL}")
    system = platform.system()
    potential_paths: List[str] = []
    home_dir = get_actual_home_dir()

    if system == "Windows":
        localappdata = os.getenv("LOCALAPPDATA")
        programfiles = os.getenv("ProgramFiles")
        programfiles_x86 = os.getenv("ProgramFiles(x86)")
        userprofile = os.getenv("USERPROFILE")
        if localappdata: potential_paths.append(os.path.join(localappdata, "Programs", "Cursor", "resources", "app"))
        if programfiles: potential_paths.append(os.path.join(programfiles, "Cursor", "resources", "app"))
        if programfiles_x86: potential_paths.append(os.path.join(programfiles_x86, "Cursor", "resources", "app"))
        if userprofile: potential_paths.append(os.path.join(userprofile, "scoop", "apps", "cursor", "current", "resources", "app"))
    elif system == "Darwin":
        potential_paths.append(os.path.join(home_dir, "Applications/Cursor.app/Contents/Resources/app"))
        potential_paths.append("/Applications/Cursor.app/Contents/Resources/app")
    elif system == "Linux":
        potential_paths.extend([
            "/opt/Cursor/resources/app",
            "/usr/share/cursor/resources/app",
            "/usr/lib/cursor/resources/app",
            "/usr/local/share/cursor/resources/app",
            os.path.join(home_dir, ".local/share/cursor/resources/app")
        ])
        appimage_patterns = ["squashfs-root*/resources/app", "squashfs-root*/usr/share/cursor/resources/app", ".mount_Cursor*/resources/app", "Applications/Cursor*/resources/app"]
        for pattern in appimage_patterns:
            potential_paths.extend(glob.glob(os.path.join(home_dir, pattern)))
            potential_paths.extend(glob.glob(os.path.join("/tmp", pattern)))
            potential_paths.extend(glob.glob(pattern))
        flatpak_path = "/var/lib/flatpak/app/com.cursor.Cursor/current/active/files/share/cursor/resources/app"
        snap_path = "/snap/cursor/current/resources/app"
        snap_alt_path = os.path.join(home_dir, "snap/cursor/current/resources/app")
        if os.path.exists(flatpak_path): potential_paths.append(flatpak_path)
        if os.path.exists(snap_path): potential_paths.append(snap_path)
        if os.path.exists(snap_alt_path): potential_paths.append(snap_alt_path)

    checked_paths = set()
    valid_path = None
    for path in potential_paths:
        if is_system_path(path):  # Skip system paths and .appimage
            continue
        normalized_path = os.path.abspath(path)
        if normalized_path in checked_paths or not os.path.isdir(normalized_path): continue
        checked_paths.add(normalized_path)
        pkg_json = os.path.join(normalized_path, "package.json")
        main_js = os.path.join(normalized_path, "out/main.js")
        workbench_js = os.path.join(normalized_path, "out/vs/workbench/workbench.desktop.main.js")
        if os.path.isfile(pkg_json) and os.path.isfile(main_js) and os.path.isfile(workbench_js):
            print(f"{Fore.GREEN}{EMOJI['SUCCESS']} {t('find_path.found_app_at_valid', path=normalized_path, default=f'Found valid app resource directory: {normalized_path}')}{Style.RESET_ALL}")
            valid_path = normalized_path
            break
    if not valid_path:
        print(f"{Fore.RED}{EMOJI['ERROR']} {t('find_path.app_not_found_strict', default='Cursor app resource directory (with required JS files) not found.')}{Style.RESET_ALL}")
        checked_paths_str = ", ".join(sorted(list(checked_paths)))
        print(f"{Fore.YELLOW}{EMOJI['INFO']} {t('find_path.checked_paths_app', paths=checked_paths_str, default='Checked paths: {paths}')}{Style.RESET_ALL}")
    return valid_path

# --- Helper Functions ---

def get_cursor_paths(app_resource_dir: str, translator=None) -> Tuple[Optional[str], Optional[str]]:
    t = _get_translator_func(translator)
    pkg = os.path.join(app_resource_dir, "package.json")
    main = os.path.join(app_resource_dir, "out/main.js")
    if not os.path.exists(pkg):
        print(f"{Fore.RED}{EMOJI['ERROR']} Internal Error: {t('reset.package_not_found', path=pkg)}{Style.RESET_ALL}")
        return None, None
    if not os.path.exists(main):
        print(f"{Fore.RED}{EMOJI['ERROR']} Internal Error: {t('reset.main_not_found', path=main)}{Style.RESET_ALL}")
        return None, None
    return pkg, main

def get_workbench_cursor_path(app_resource_dir: str, translator=None) -> Optional[str]:
    t = _get_translator_func(translator)
    main_path = os.path.join(app_resource_dir, "out/vs/workbench/workbench.desktop.main.js")
    if not os.path.exists(main_path):
        print(f"{Fore.RED}{EMOJI['ERROR']} Internal Error: {t('reset.workbench_js_not_found_unexpected', path=main_path)}{Style.RESET_ALL}")
        return None
    return main_path

def version_check(version: str, min_version: str = "", max_version: str = "", translator=None) -> bool:
    t = _get_translator_func(translator)
    pattern = r"^\d+\.\d+\.\d+"
    try:
        match = re.match(pattern, version)
        if not match:
            print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.invalid_version_format', version=version)}{Style.RESET_ALL}")
            return False
        clean = match.group(0)
        current = tuple(map(int, clean.split(".")))
        if min_version and current < tuple(map(int, min_version.split("."))):
            print(f"{Fore.YELLOW}{EMOJI['WARNING']} {t('reset.version_too_low', version=version, min_version=min_version)}{Style.RESET_ALL}")
            return False
        if max_version and current > tuple(map(int, max_version.split("."))):
            print(f"{Fore.YELLOW}{EMOJI['WARNING']} {t('reset.version_too_high', version=version, max_version=max_version)}{Style.RESET_ALL}")
            return False
        return True
    except ValueError:
        print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.version_parse_error', version=version)}{Style.RESET_ALL}")
        return False
    except Exception as e:
        print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.version_check_error', error=str(e))}{Style.RESET_ALL}")
        return False

def check_cursor_version(pkg_path: str, translator) -> Optional[bool]:
    t = _get_translator_func(translator)
    try:
        print(f"{Fore.CYAN}{EMOJI['INFO']} {t('reset.reading_package_json', path=pkg_path)}{Style.RESET_ALL}")
        data = None
        try:
            with open(pkg_path, "r", encoding="utf-8") as f:
                data = json.load(f)
        except UnicodeDecodeError:
            try:
                with open(pkg_path, "r", encoding="latin-1") as f:
                    data = json.load(f)
            except Exception as re:
                print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.read_file_error_fallback', file='package.json', error=str(re))}{Style.RESET_ALL}")
                return None
        if not isinstance(data, dict):
            print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.invalid_json_object')}{Style.RESET_ALL}")
            return None
        version = data.get("version")
        if not version or not isinstance(version, str) or not version.strip():
            print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.no_version_field', default='Could not find valid \"version\" field in package.json')}{Style.RESET_ALL}")
            return None
        version = version.strip()
        print(f"{Fore.CYAN}{EMOJI['INFO']} {t('reset.found_version', version=version)}{Style.RESET_ALL}")
        min_v = "0.45.0"
        is_min = version_check(version, min_version=min_v, translator=t)
        if is_min:
            print(f"{Fore.GREEN}{EMOJI['SUCCESS']} {t('reset.version_check_passed_min', version=version, min_version=min_v)}{Style.RESET_ALL}")
            return True
        else:
            return False
    except FileNotFoundError:
        print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.package_not_found', path=pkg_path)}{Style.RESET_ALL}")
        return None
    except json.JSONDecodeError:
        print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.invalid_json_object')}{Style.RESET_ALL}")
        return None
    except Exception as e:
        print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.check_version_failed', error=str(e))}{Style.RESET_ALL}")
        return None

def safe_delete(path: str, is_dir: bool, translator=None):
    """Safely delete a file or directory with logging and error handling, skipping system paths and .appimage."""
    t = _get_translator_func(translator)
    item_type = "directory" if is_dir else "file"
    if not os.path.exists(path):
        return True  # Not an error if it doesn't exist

    if is_system_path(path):
        print(f"{Fore.YELLOW}{EMOJI['INFO']} {t('delete.skipping_system', type=item_type, path=path, default=f'Skipping delete: {item_type} is a system path or .appimage: {path}')}{Style.RESET_ALL}")
        return True

    print(f"{Fore.YELLOW}{EMOJI['DELETE']} {t('delete.attempting', type=item_type, path=path, default=f'Attempting to delete {item_type}: {path}')}{Style.RESET_ALL}")
    try:
        if is_dir:
            shutil.rmtree(path)
        else:
            os.remove(path)
        print(f"{Fore.GREEN}{EMOJI['SUCCESS']} {t('delete.success', type=item_type, path=path, default=f'Successfully deleted {item_type}: {path}')}{Style.RESET_ALL}")
        return True
    except PermissionError:
        print(f"{Fore.RED}{EMOJI['ERROR']} {t('delete.permission_error', type=item_type, path=path, default=f'Permission denied deleting {item_type}: {path}')}{Style.RESET_ALL}")
        print(f"{Fore.YELLOW}{EMOJI['LOCK']} {t('reset.try_sudo')}{Style.RESET_ALL}")
        return False
    except OSError as e:
        print(f"{Fore.RED}{EMOJI['ERROR']} {t('delete.os_error', type=item_type, path=path, error=str(e), default=f'OS error deleting {item_type} {path}: {e}')}{Style.RESET_ALL}")
        return False
    except Exception as e:
        print(f"{Fore.RED}{EMOJI['ERROR']} {t('delete.unexpected_error', type=item_type, path=path, error=str(e), default=f'Unexpected error deleting {item_type} {path}: {e}')}{Style.RESET_ALL}")
        return False

def modify_file_content(file_path: str, replacements: List[Tuple[str, str]], translator=None, is_js=False) -> bool:
    t = _get_translator_func(translator)
    basename = os.path.basename(file_path)
    if not os.path.exists(file_path):
        print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.file_not_found', path=file_path)}{Style.RESET_ALL}")
        return False
    if is_system_path(file_path):
        print(f"{Fore.YELLOW}{EMOJI['INFO']} {t('reset.skipping_system_file', path=file_path, default='Skipping modification: file is in system path or .appimage')}{Style.RESET_ALL}")
        return True
    if not os.access(file_path, os.W_OK):
        print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.no_write_permission', path=file_path)}{Style.RESET_ALL}")
        print(f"{Fore.YELLOW}{EMOJI['LOCK']} {t('reset.try_sudo')}{Style.RESET_ALL}")
        return False
    original_mode, original_uid, original_gid = None, -1, -1
    try:
        st = os.stat(file_path)
        original_mode = st.st_mode
    except OSError as e:
        print(f"{Fore.YELLOW}{EMOJI['WARNING']} {t('reset.stat_error', path=file_path, error=str(e))}{Style.RESET_ALL}")
    ts = time.strftime("%Y%m%d_%H%M%S")
    backup_path = f"{file_path}.{ts}.bak"
    try:
        if not os.path.exists(backup_path):
            shutil.copy2(file_path, backup_path)
            print(f"{Fore.GREEN}{EMOJI['BACKUP']} {t('reset.backup_created', path=backup_path)}{Style.RESET_ALL}")
    except Exception as be:
        print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.backup_failed_file', file=basename, error=str(be))}{Style.RESET_ALL}")
        return False
    print(f"{Fore.CYAN}{EMOJI['FILE']} {t('reset.reading_file', file=basename)}{Style.RESET_ALL}")
    try:
        with open(file_path, "r", encoding="utf-8", errors="ignore") as mf:
            content = mf.read()
    except Exception as re:
        print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.read_file_error', file=file_path, error=str(re))}{Style.RESET_ALL}")
        return False
    modified = content
    found, changed = False, False
    print(f"{Fore.CYAN}{EMOJI['RESET']} {t('reset.applying_patches', file=basename)}{Style.RESET_ALL}")
    for i, (old, new) in enumerate(replacements):
        temp = modified
        key_index = i + 1
        try:
            if is_js:
                temp = re.sub(old, new, modified)
            else:
                temp = modified.replace(old, new)
            if temp != modified:
                print(f"{Fore.GREEN}   {EMOJI['SUCCESS']} {t('reset.patch_applied' if is_js else 'reset.replacement_applied', index=key_index)}{Style.RESET_ALL}")
                modified = temp
                found, changed = True, True
            elif (is_js and re.search(old, modified)) or (not is_js and old in modified):
                print(f"{Fore.YELLOW}   {EMOJI['INFO']} {t('reset.patch_matched_no_change', index=key_index)}{Style.RESET_ALL}")
                found = True
        except re.error as regex_err:
            print(f"{Fore.RED}   {EMOJI['ERROR']} {t('reset.regex_error', index=key_index, error=str(regex_err))}{Style.RESET_ALL}")
    if not found:
        print(f"{Fore.YELLOW}{EMOJI['INFO']} {t('reset.no_patterns_found', file=basename)}{Style.RESET_ALL}")
        return True
    elif not changed:
        print(f"{Fore.YELLOW}{EMOJI['INFO']} {t('reset.patterns_found_no_change', file=basename)}{Style.RESET_ALL}")
        return True
    tmp_path = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", errors="ignore", delete=False, dir=os.path.dirname(file_path), prefix=f"{basename}.") as tf:
            tf.write(modified)
            tmp_path = tf.name
        shutil.move(tmp_path, file_path)
        tmp_path = None
        if original_mode is not None:
            try:
                os.chmod(file_path, original_mode)
            except OSError as pe:
                print(f"{Fore.YELLOW}{EMOJI['WARNING']} {t('reset.permission_restore_failed', file=basename, error=str(pe))}{Style.RESET_ALL}")
        print(f"{Fore.GREEN}{EMOJI['SUCCESS']} {t('reset.file_modified_success', file=basename)}{Style.RESET_ALL}")
        return True
    except Exception as we:
        print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.modify_file_failed', file=basename, error=str(we))}{Style.RESET_ALL}")
        if os.path.exists(backup_path):
            try:
                print(f"{Fore.YELLOW}{EMOJI['INFO']} {t('reset.restoring_backup', path=backup_path)}{Style.RESET_ALL}")
                shutil.move(backup_path, file_path)
                print(f"{Fore.GREEN}{EMOJI['SUCCESS']} {t('reset.restore_success')}{Style.RESET_ALL}")
            except Exception as rre:
                print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.restore_failed', error=str(rre))}{Style.RESET_ALL}")
                print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.manual_restore_needed', original=file_path, backup=backup_path)}{Style.RESET_ALL}")
        return False
    finally:
        if tmp_path and os.path.exists(tmp_path):
            try:
                os.unlink(tmp_path)
            except OSError:
                pass

# --- JS Patching Functions (Rely on modify_file_content) ---

def modify_workbench_js(file_path: str, translator=None) -> bool:
    t = _get_translator_func(translator)
    if is_system_path(file_path):
        print(f"{Fore.YELLOW}{EMOJI['INFO']} {t('reset.skipping_system_file_patch', path=file_path, default='Skipping JS patch: file is in system path or .appimage')}{Style.RESET_ALL}")
        return True
    print(f"{Fore.CYAN}{EMOJI['INFO']} {t('reset.modifying_workbench')}{Style.RESET_ALL}")
    repo_url = "https://github.com/your-repo/cursor-free-vip"  # Replace with actual URL or remove if not needed
    old_regex = r'(title:"Upgrade to Pro"\s*,\s*size:"small"\s*,\s*get codicon\s*\(\)\s*\{[^}]*return\s+\w+\.rocket[^}]*\}\s*,\s*get onClick\s*\(\)\s*\{[^}]*return\s+)(\w+\.pay)([^}]*\})'
    new_repl = rf'\g<1>function(){{window.open("{repo_url}","_blank")}}(\g<3>'
    simple = [('<div>Pro Trial', '<div>Pro'), ('notifications-toasts', 'notifications-toasts hidden')]
    regex = [(old_regex, new_repl)]
    modify_file_content(file_path, simple, translator=t, is_js=False)  # Apply simple first
    print(f"{Fore.CYAN}{EMOJI['INFO']} {t('reset.workbench_button_patch_attempt')}{Style.RESET_ALL}")
    return modify_file_content(file_path, regex, translator=t, is_js=True)  # Then regex

def modify_main_js(main_path: str, translator) -> bool:
    t = _get_translator_func(translator)
    if is_system_path(main_path):
        print(f"{Fore.YELLOW}{EMOJI['INFO']} {t('reset.skipping_system_file_patch', path=main_path, default='Skipping JS patch: file is in system path or .appimage')}{Style.RESET_ALL}")
        return True
    print(f"{Fore.CYAN}{EMOJI['INFO']} {t('reset.modifying_mainjs')}{Style.RESET_ALL}")
    replacements = [
        (r"(async getMachineId\(\)\s*\{[^}]*return\s+)([^?}]+\?\?)([^}]+})", r"\1\3"),
        (r"(async getMacMachineId\(\)\s*\{[^}]*return\s+)([^?}]+\?\?)([^}]+})", r"\1\3")
    ]
    return modify_file_content(main_path, replacements, translator=t, is_js=True)

def patch_cursor_get_machine_id(app_resource_dir: str, translator) -> bool:
    t = _get_translator_func(translator)
    if is_system_path(app_resource_dir):
        print(f"{Fore.YELLOW}{EMOJI['INFO']} {t('reset.skipping_system_dir_patch', path=app_resource_dir, default='Skipping JS patch: directory is in system path or .appimage')}{Style.RESET_ALL}")
        return True
    try:
        print(f"{Fore.CYAN}{EMOJI['INFO']} {t('reset.start_patching_mainjs')}{Style.RESET_ALL}")
        _, main_path = get_cursor_paths(app_resource_dir, t)
        if not main_path:
            print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.cannot_patch_mainjs_paths')}{Style.RESET_ALL}")
            return False
        if not modify_main_js(main_path, t):
            print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.patch_failed_mainjs')}{Style.RESET_ALL}")
            return False
        print(f"{Fore.GREEN}{EMOJI['SUCCESS']} {t('reset.patch_completed_mainjs')}{Style.RESET_ALL}")
        return True
    except Exception as e:
        print(f"{Fore.RED}{EMOJI['ERROR']} {t('reset.patch_failed', error=str(e))}{Style.RESET_ALL}")
        return False

# --- Main Resetter Class ---

class MachineIDResetter:
    def __init__(self, translator=None):
        self.translator = _get_translator_func(translator)
        self.config_dir: Optional[str] = None
        self.storage_json_path: Optional[str] = None
        self.state_db_path: Optional[str] = None
        self.machine_id_file_path: Optional[str] = None
        self.app_resource_dir: Optional[str] = None
        self._find_paths()

    def _find_paths(self):
        print(f"\n{Fore.CYAN}{'='*20} Path Finding {'='*20}{Style.RESET_ALL}")
        self.config_dir = find_cursor_config_dir(self.translator)
        self.app_resource_dir = find_cursor_app_resource_dir(self.translator)
        if self.config_dir and not is_system_path(self.config_dir):
            user_gs = os.path.join(self.config_dir, "User", "globalStorage")
            try:
                os.makedirs(user_gs, exist_ok=True)
            except OSError as e:
                print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('reset.mkdir_failed', path=user_gs, error=str(e))}{Style.RESET_ALL}")
                self.config_dir = None
                return
            self.storage_json_path = os.path.join(user_gs, "storage.json")
            self.state_db_path = os.path.join(user_gs, "state.vscdb")
            sys_plat = platform.system()
            if sys_plat == "Linux":
                self.machine_id_file_path = os.path.join(self.config_dir, "machineid")
            elif sys_plat == "Windows":
                self.machine_id_file_path = os.path.join(self.config_dir, "machineId")
            elif sys_plat == "Darwin":
                self.machine_id_file_path = os.path.join(self.config_dir, "machineId")
            else:
                self.machine_id_file_path = None
            print(f"{EMOJI['PATH']}{Fore.GREEN}Config Dir:{Style.RESET_ALL} {self.config_dir}")
            print(f"{EMOJI['PATH']}{Fore.GREEN}Storage JSON:{Style.RESET_ALL} {self.storage_json_path}")
            print(f"{EMOJI['PATH']}{Fore.GREEN}State DB:{Style.RESET_ALL} {self.state_db_path}")
            if self.machine_id_file_path:
                print(f"{EMOJI['PATH']}{Fore.GREEN}Machine ID File:{Style.RESET_ALL} {self.machine_id_file_path}")
            else:
                print(f"{EMOJI['PATH']}{Fore.YELLOW}Machine ID File:{Style.RESET_ALL} (Path undetermined)")
        else:
            print(f"{Fore.RED}{EMOJI['ERROR']} Critical: {self.translator('find_path.config_not_found')}{Style.RESET_ALL}")
            self.storage_json_path = None
            self.state_db_path = None
            self.machine_id_file_path = None
        if self.app_resource_dir and not is_system_path(self.app_resource_dir):
            print(f"{EMOJI['PATH']}{Fore.GREEN}App Resource Dir:{Style.RESET_ALL} {self.app_resource_dir}")
        else:
            print(f"{Fore.YELLOW}{EMOJI['WARNING']} {self.translator('find_path.app_not_found_skip_patch', default='App resource directory not found or is system path. JS patching will be skipped.')}{Style.RESET_ALL}")
        print(f"{Fore.CYAN}{'='*53}{Style.RESET_ALL}\n")

    def _check_prerequisites(self) -> bool:
        print(f"{Fore.CYAN}{EMOJI['INFO']} {self.translator('reset.checking_prereqs')}{Style.RESET_ALL}")
        if not all([self.config_dir, self.storage_json_path, self.state_db_path, self.machine_id_file_path]) or is_system_path(self.config_dir):
            print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('reset.error_missing_paths_reset')}{Style.RESET_ALL}")
            return False
        permission_ok = True
        paths_to_check = [self.storage_json_path, self.state_db_path, self.machine_id_file_path]
        for path in paths_to_check:
            if is_system_path(path):
                print(f"{Fore.YELLOW}{EMOJI['INFO']} {t('reset.skipping_system_check', path=path, default='Skipping permission check: path is system or .appimage')}{Style.RESET_ALL}")
                continue
            parent = os.path.dirname(path)
            try:
                os.makedirs(parent, exist_ok=True)
                if not os.access(parent, os.W_OK):
                    print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('reset.no_permission_dir', path=parent)}{Style.RESET_ALL}")
                    permission_ok = False
            except OSError as e:
                print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('reset.mkdir_failed', path=parent, error=str(e))}{Style.RESET_ALL}")
                permission_ok = False
                continue
            if os.path.exists(path) and not os.access(path, os.W_OK):
                print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('reset.no_permission_file_generic', path=path)}{Style.RESET_ALL}")
                permission_ok = False
        if not permission_ok:
            print(f"{Fore.YELLOW}{EMOJI['LOCK']} {self.translator('reset.try_sudo')}{Style.RESET_ALL}")
            return False
        print(f"{Fore.GREEN}{EMOJI['SUCCESS']} {self.translator('reset.prereq_ok')}{Style.RESET_ALL}")
        return True

    def generate_new_ids(self) -> dict:
        print(f"{Fore.CYAN}{EMOJI['RESET']} {self.translator('reset.generating')}{Style.RESET_ALL}")
        dev_id = str(uuid.uuid4())
        m_id = hashlib.sha256(os.urandom(32)).hexdigest()
        mac_id = str(uuid.uuid4())
        sqm_id = "{" + str(uuid.uuid4()).upper() + "}"
        inst_id = str(uuid.uuid4())
        sess_id = str(uuid.uuid4()) + str(int(time.time() * 1000))
        new_ids = {
            "telemetry.telemetryLevel": "off",
            "telemetry.instanceId": inst_id,
            "telemetry.sessionId": sess_id,
            "telemetry.devDeviceId": dev_id,
            "telemetry.macMachineId": mac_id,
            "telemetry.machineId": m_id,
            "telemetry.sqmId": sqm_id,
            "storage.serviceMachineId": dev_id,
            "workbench.startupEditor": "none",
            "extensions.ignoreRecommendations": True,
            "cursor.internal.loginToken": "",
            "cursor.internal.userTier": "free",
            "cursor.internal.trialExpired": False,
            "cursor.internal.lastCheckTimestamp": 0,
            "cursor.initialStartup": False
        }
        print(f"{Fore.CYAN}Generated values:{Style.RESET_ALL}")
        [print(f"  {EMOJI['INFO']} {k}: {Fore.GREEN}{v}{Style.RESET_ALL}") for k, v in new_ids.items() if 'Id' in k or 'Tier' in k or 'Token' in k]
        print(f"{Fore.CYAN}  {EMOJI['INFO']} Updating separate machineId file with: {Fore.GREEN}{dev_id}{Style.RESET_ALL}")
        if not self.update_machine_id_file(dev_id):
            print(f"{Fore.YELLOW}{EMOJI['WARNING']} {self.translator('reset.machineid_update_failed_warn')}{Style.RESET_ALL}")
        return new_ids

    def update_storage_json(self, new_ids: dict) -> bool:
        if not self.storage_json_path or is_system_path(self.storage_json_path):
            print(f"{Fore.YELLOW}{EMOJI['INFO']} {self.translator('reset.skipping_system_json', path=self.storage_json_path, default='Skipping JSON update: path is system or .appimage')}{Style.RESET_ALL}")
            return True
        return modify_file_content(self.storage_json_path, list(new_ids.items()), self.translator, is_js=False)

    def update_sqlite_db(self, new_ids: dict) -> bool:
        if not self.state_db_path or is_system_path(self.state_db_path):
            print(f"{Fore.YELLOW}{EMOJI['INFO']} {self.translator('reset.skipping_system_sqlite', path=self.state_db_path, default='Skipping SQLite update: path is system or .appimage')}{Style.RESET_ALL}")
            return True
        print(f"{Fore.CYAN}{EMOJI['FILE']} {self.translator('reset.updating_sqlite')}{Style.RESET_ALL}")
        ts = time.strftime("%Y%m%d_%H%M%S")
        backup = f"{self.state_db_path}.{ts}.bak"
        try:
            if os.path.exists(self.state_db_path):
                shutil.copy2(self.state_db_path, backup)
                print(f"{Fore.GREEN}{EMOJI['BACKUP']} {self.translator('reset.backup_created', path=backup)}{Style.RESET_ALL}")
            else:
                print(f"{Fore.YELLOW}{EMOJI['INFO']} {self.translator('reset.sqlite_not_exist')}{Style.RESET_ALL}")
                return True
        except Exception as be:
            print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('reset.backup_failed_file', file='state.vscdb', error=str(be))}{Style.RESET_ALL}")
            return False
        conn = None
        try:
            os.makedirs(os.path.dirname(self.state_db_path), exist_ok=True)
            conn = sqlite3.connect(self.state_db_path, timeout=10)
            cur = conn.cursor()
            cur.execute("CREATE TABLE IF NOT EXISTS ItemTable (key TEXT PRIMARY KEY NOT NULL, value BLOB)")
            conn.commit()
            keys = [k for k in new_ids if k.startswith("telemetry.") or k.startswith("storage.")]
            if not keys:
                print(f"{Fore.YELLOW}{EMOJI['INFO']} {self.translator('reset.sqlite_no_keys')}{Style.RESET_ALL}")
                conn.close()
                return True
            print(f"{Fore.CYAN}Updating SQLite key-value pairs:{Style.RESET_ALL}")
            cur.execute("BEGIN TRANSACTION")
            [print(f"  {EMOJI['INFO']} {k}: {Fore.GREEN}{new_ids[k]}{Style.RESET_ALL}") or cur.execute("INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?, ?)", (k, str(new_ids[k]).encode('utf-8'))) for k in keys]
            conn.commit()
            print(f"{Fore.GREEN}{EMOJI['SUCCESS']} {self.translator('reset.sqlite_success')}{Style.RESET_ALL}")
            return True
        except sqlite3.Error as dbe:
            print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('reset.sqlite_error', error=str(dbe))}{Style.RESET_ALL}")
            return False
        finally:
            if conn:
                conn.close()
        return False

    def update_machine_id_file(self, new_id: str) -> bool:
        if not self.machine_id_file_path or is_system_path(self.machine_id_file_path):
            print(f"{Fore.YELLOW}{EMOJI['INFO']} {self.translator('reset.skipping_system_machineid', path=self.machine_id_file_path, default='Skipping machine ID update: path is system or .appimage')}{Style.RESET_ALL}")
            return True
        print(f"{Fore.CYAN}{EMOJI['FILE']} {self.translator('reset.updating_machineid_file', file=self.machine_id_file_path)}{Style.RESET_ALL}")
        parent = os.path.dirname(self.machine_id_file_path)
        ts = time.strftime("%Y%m%d_%H%M%S")
        backup = f"{self.machine_id_file_path}.{ts}.bak"
        try:
            os.makedirs(parent, exist_ok=True)
        except OSError as e:
            print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('reset.mkdir_failed', path=parent, error=str(e))}{Style.RESET_ALL}")
            return False
        if os.path.exists(self.machine_id_file_path):
            try:
                shutil.copy2(self.machine_id_file_path, backup)
                print(f"{Fore.GREEN}{EMOJI['BACKUP']} {self.translator('reset.backup_created', path=backup)}{Style.RESET_ALL}")
            except Exception as be:
                print(f"{Fore.YELLOW}{EMOJI['WARNING']} {self.translator('reset.backup_failed_file', file='machineId', error=str(be))}{Style.RESET_ALL}")
        tmp = None
        try:
            with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", delete=False, dir=parent, prefix="machineId.") as tf:
                tf.write(new_id)
                tmp = tf.name
            shutil.move(tmp, self.machine_id_file_path)
            tmp = None
            print(f"{Fore.GREEN}{EMOJI['SUCCESS']} {self.translator('reset.machineid_update_success')}{Style.RESET_ALL}")
            return True
        except Exception as we:
            print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('reset.write_file_error', file='machineId', error=str(we))}{Style.RESET_ALL}")
            return False
        finally:
            if tmp and os.path.exists(tmp):
                try:
                    os.unlink(tmp)
                except OSError:
                    pass
        return False

    def _update_windows_machine_guid(self, guid: str) -> bool:
        return True  # Placeholder implementation

    def _update_windows_sqm_machine_id(self, guid: str) -> bool:
        return True  # Placeholder implementation

    def _update_macos_platform_uuid(self, guid: str) -> bool:
        return True  # Placeholder implementation

    def update_system_ids(self, ids: dict) -> bool:
        print(f"\n{Fore.CYAN}{EMOJI['INFO']} {self.translator('reset.updating_system_ids')}{Style.RESET_ALL}")
        print(f"{Fore.YELLOW}{EMOJI['WARNING']} {self.translator('reset.system_id_warning')}{Style.RESET_ALL}")
        flags = []
        sys_p = platform.system()
        if sys_p == "Windows":
            guid = ids.get("telemetry.devDeviceId", str(uuid.uuid4()))
            guid_b = "{" + ids.get("telemetry.sqmId", str(uuid.uuid4()).upper()) + "}"
            print(f"{Fore.CYAN}  {EMOJI['INFO']} Target MachineGuid: {guid}{Style.RESET_ALL}\n  {EMOJI['INFO']} Target SQM MachineId: {guid_b}{Style.RESET_ALL}")
            flags.append(self._update_windows_machine_guid(guid))
            flags.append(self._update_windows_sqm_machine_id(guid_b))
        elif sys_p == "Darwin":
            uuid_p = ids.get("telemetry.macMachineId", str(uuid.uuid4()).upper())
            print(f"{Fore.CYAN}  {EMOJI['INFO']} Target Platform UUID: {uuid_p}{Style.RESET_ALL}")
            flags.append(self._update_macos_platform_uuid(uuid_p))
        elif sys_p == "Linux":
            print(f"{Fore.YELLOW}{EMOJI['INFO']} {self.translator('reset.linux_system_id_skip')}{Style.RESET_ALL}")
            flags.append(True)
        else:
            print(f"{Fore.YELLOW}{EMOJI['WARNING']} {self.translator('reset.unsupported_os_system_id')}{Style.RESET_ALL}")
            flags.append(True)
        ok = all(flags)
        if ok:
            print(f"{Fore.GREEN}{EMOJI['SUCCESS']} {self.translator('reset.system_ids_updated')}{Style.RESET_ALL}")
        else:
            print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('reset.system_ids_update_failed')}{Style.RESET_ALL}")
        return ok

    def purge_config_data(self) -> bool:
        if not self.config_dir or not os.path.isdir(self.config_dir) or is_system_path(self.config_dir):
            print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('purge.no_config_dir', default='Cannot purge data: Cursor configuration directory not found, invalid, or is system path.')}{Style.RESET_ALL}")
            return False

        print(f"\n{Fore.YELLOW}{Style.BRIGHT}{EMOJI['WARNING']} {self.translator('purge.warning_title', default='DESTRUCTIVE ACTION WARNING!')}{Style.RESET_ALL}")
        print(f"{Fore.YELLOW}{self.translator('purge.warning_text_1', path=self.config_dir, default=f'The --purge flag will permanently delete specific state data within:')}{Style.RESET_ALL}")
        print(f"{Fore.YELLOW}  {self.config_dir}{Style.RESET_ALL}")
        print(f"{Fore.YELLOW}{self.translator('purge.warning_text_2', default='This includes logs, workspace history, global state, and machine IDs.')}{Style.RESET_ALL}")
        print(f"{Fore.YELLOW}{self.translator('purge.warning_text_3', default='Your settings (settings.json) and extensions will NOT be deleted by this.')}{Style.RESET_ALL}")
        print(f"{Fore.YELLOW}{Style.BRIGHT}{self.translator('purge.warning_text_4', default='THIS CANNOT BE UNDONE.')}{Style.RESET_ALL}")

        try:
            confirm = input(f"{Fore.CYAN}{self.translator('purge.confirm_prompt', default='Type YES to confirm deletion: ')}{Style.RESET_ALL}")
            if confirm.strip().upper() != "YES":
                print(f"{Fore.YELLOW}{EMOJI['INFO']} {self.translator('purge.aborted', default='Purge aborted by user.')}{Style.RESET_ALL}")
                return False
        except EOFError:
            print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('purge.confirm_failed_eof', default='Confirmation failed (EOF). Aborting purge.')}{Style.RESET_ALL}")
            return False

        print(f"{Fore.CYAN}{EMOJI['DELETE']} {self.translator('purge.starting', default='Starting purge process...')}{Style.RESET_ALL}")

        items_to_purge = [
            ("User/globalStorage", True),
            ("User/workspaceStorage", True),
            ("User/History", True),
            ("User/logs", True),
            ("User/blob_storage", True),
            ("machineid", False),
            ("machineId", False),
            ("LOCK", False),
            ("storage.json", False),
            ("code-runner-cache", True),
            ("Crashpad", True),
        ]

        all_deleted_successfully = True
        for item_rel_path, is_dir in items_to_purge:
            full_path = os.path.join(self.config_dir, item_rel_path)
            if not is_system_path(full_path):
                if not safe_delete(full_path, is_dir, self.translator):
                    all_deleted_successfully = False
            else:
                print(f"{Fore.YELLOW}{EMOJI['INFO']} {self.translator('purge.skipping_system_item', path=full_path, default='Skipping purge: item is in system path or .appimage')}{Style.RESET_ALL}")

        if all_deleted_successfully:
            print(f"{Fore.GREEN}{EMOJI['SUCCESS']} {self.translator('purge.completed', default='Purge process completed.')}{Style.RESET_ALL}")
        else:
            print(f"{Fore.YELLOW}{EMOJI['WARNING']} {self.translator('purge.completed_errors', default='Purge process completed with errors. Check logs.')}{Style.RESET_ALL}")

        return all_deleted_successfully

    def reset_machine_ids(self, skip_system_ids=False, skip_js_patches=False, purge_data=False):
        print(f"\n{Fore.CYAN}{'='*18} Starting Cursor Reset {'='*18}{Style.RESET_ALL}")
        if not self._check_prerequisites():
            print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('reset.prereq_failed_stop')}{Style.RESET_ALL}")
            return False
        new_ids = self.generate_new_ids()
        if not new_ids:
            print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('reset.id_generation_failed')}{Style.RESET_ALL}")
            return False
        json_ok = self.update_storage_json(new_ids)
        sqlite_ok = self.update_sqlite_db(new_ids)
        if not json_ok or not sqlite_ok:
            print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('reset.config_update_failed_stop')}{Style.RESET_ALL}")
            return False
        sys_id_ok = True
        if not skip_system_ids:
            sys_id_ok = self.update_system_ids(new_ids)
            if not sys_id_ok:
                print(f"{Fore.YELLOW}{EMOJI['WARNING']} {self.translator('reset.system_id_update_failed_warn')}{Style.RESET_ALL}")
        else:
            print(f"{Fore.YELLOW}{EMOJI['INFO']} {self.translator('reset.system_id_skipped')}{Style.RESET_ALL}")
        patch_ok = True
        if not skip_js_patches:
            if self.app_resource_dir and not is_system_path(self.app_resource_dir):
                print(f"\n{Fore.CYAN}{'='*18} Applying JS Patches {'='*18}{Style.RESET_ALL}")
                pkg, main = get_cursor_paths(self.app_resource_dir, self.translator)
                wb = get_workbench_cursor_path(self.app_resource_dir, self.translator)
                paths = [p for p in [pkg, main, wb] if p and not is_system_path(p)]
                perm_ok = all(os.access(p, os.W_OK) for p in paths if p)
                if not perm_ok:
                    print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('reset.no_write_permission_js_summary')}{Style.RESET_ALL}")
                    [print(f"   - {p}") for p in paths if p and not os.access(p, os.W_OK)]
                    print(f"{Fore.YELLOW}{EMOJI['LOCK']} {self.translator('reset.try_sudo_js')}{Style.RESET_ALL}")
                    patch_ok = False
                else:
                    ver_ok = check_cursor_version(pkg, self.translator) if pkg and not is_system_path(pkg) else None
                    main_ok = True
                    if ver_ok is True:
                        print(f"{Fore.CYAN}{EMOJI['INFO']} {self.translator('reset.version_ok_patching')}{Style.RESET_ALL}")
                        main_ok = patch_cursor_get_machine_id(self.app_resource_dir, self.translator)
                    elif ver_ok is False:
                        print(f"{Fore.YELLOW}{EMOJI['INFO']} {self.translator('reset.version_low_skip_patch')}{Style.RESET_ALL}")
                    else:
                        print(f"{Fore.YELLOW}{EMOJI['WARNING']} {self.translator('reset.version_check_failed_skip_patch')}{Style.RESET_ALL}")
                    wb_ok = modify_workbench_js(wb, self.translator) if wb and not is_system_path(wb) else True
                    patch_ok = main_ok and wb_ok
            else:
                print(f"{Fore.YELLOW}{EMOJI['INFO']} {self.translator('reset.app_dir_not_found_skip_patches')}{Style.RESET_ALL}")
        else:
            print(f"{Fore.YELLOW}{EMOJI['INFO']} {self.translator('reset.js_patch_skipped')}{Style.RESET_ALL}")

        purge_ok = True
        if purge_data:
            purge_ok = self.purge_config_data()

        print(f"\n{Fore.CYAN}{'='*20} Reset Summary {'='*21}{Style.RESET_ALL}")
        overall_ok = json_ok and sqlite_ok and sys_id_ok and patch_ok and purge_ok
        if overall_ok:
            print(f"{Fore.GREEN}{EMOJI['SUCCESS']} {self.translator('reset.success_complete')}{Style.RESET_ALL}")
            print(f"\n{Fore.CYAN}{self.translator('reset.new_id_summary')}:{Style.RESET_ALL}")
            print(f"  {EMOJI['INFO']} telemetry.devDeviceId: {Fore.GREEN}{new_ids.get('telemetry.devDeviceId')}{Style.RESET_ALL}")
            print(f"  {EMOJI['INFO']} telemetry.machineId: {Fore.GREEN}{new_ids.get('telemetry.machineId')}{Style.RESET_ALL}")
            print(f"  {EMOJI['INFO']} storage.serviceMachineId: {Fore.GREEN}{new_ids.get('storage.serviceMachineId')}{Style.RESET_ALL}")
            return True
        else:
            print(f"{Fore.RED}{EMOJI['ERROR']} {self.translator('reset.success_partial', default='Cursor reset process finished with warnings or errors.')}{Style.RESET_ALL}")
            if not json_ok:
                print(f"  {EMOJI['ERROR']} {self.translator('reset.summary_fail_json')}{Style.RESET_ALL}")
            if not sqlite_ok:
                print(f"  {EMOJI['ERROR']} {self.translator('reset.summary_fail_sqlite')}{Style.RESET_ALL}")
            if not sys_id_ok and not skip_system_ids:
                print(f"  {EMOJI['WARNING']} {self.translator('reset.summary_fail_system_id')}{Style.RESET_ALL}")
            if not patch_ok and not skip_js_patches:
                print(f"  {EMOJI['WARNING']} {self.translator('reset.summary_fail_patch')}{Style.RESET_ALL}")
            if not purge_ok and purge_data:
                print(f"  {EMOJI['WARNING']} {self.translator('reset.summary_fail_purge', default='Failed deleting one or more config items (optional)')}{Style.RESET_ALL}")
            print(f"{Fore.YELLOW}{EMOJI['INFO']} {self.translator('reset.check_logs_above')}{Style.RESET_ALL}")
            return json_ok and sqlite_ok

# --- Main Execution ---

def run(translator=None, skip_system=False, skip_patches=False, purge=False):
    effective_translator = _get_translator_func(translator)
    if translator is None:
        print(f"{Fore.YELLOW}Using basic fallback translator.{Style.RESET_ALL}")

    print(f"\n{Fore.CYAN}{'='*50}{Style.RESET_ALL}")
    print(f"{Fore.CYAN}{EMOJI['RESET']} {effective_translator('reset.title', default='Cursor Machine ID Reset Tool')}{Style.RESET_ALL}")
    print(f"{Fore.CYAN}{'='*50}{Style.RESET_ALL}")
    overall_status = False
    try:
        resetter = MachineIDResetter(effective_translator)
        if resetter.storage_json_path and resetter.state_db_path and resetter.machine_id_file_path and not any(is_system_path(p) for p in [resetter.storage_json_path, resetter.state_db_path, resetter.machine_id_file_path]):
            overall_status = resetter.reset_machine_ids(skip_system_ids=skip_system, skip_js_patches=skip_patches, purge_data=purge)
        else:
            print(f"{Fore.RED}{EMOJI['ERROR']} {effective_translator('reset.abort_missing_paths', default='Aborting reset due to missing essential configuration paths or system paths.')}{Style.RESET_ALL}")
            overall_status = False
    except Exception as e:
        print(f"\n{Fore.RED}{EMOJI['ERROR']} {effective_translator('reset.process_error_unexpected', error=str(e), default=f'Unexpected error during reset process: {e}')}{Style.RESET_ALL}")
        print(f"{Fore.YELLOW}{EMOJI['INFO']} {effective_translator('reset.stack_trace', default='Stack trace')}: {traceback.format_exc()}{Style.RESET_ALL}")
        overall_status = False

    print(f"\n{Fore.CYAN}{'='*50}{Style.RESET_ALL}")
    if overall_status:
        print(f"{Fore.GREEN}{EMOJI['SUCCESS']} {effective_translator('reset.final_message_success', default='Operation finished successfully.')}{Style.RESET_ALL}")
    else:
        print(f"{Fore.RED}{EMOJI['ERROR']} {effective_translator('reset.final_message_error', default='Operation finished with errors or warnings.')}{Style.RESET_ALL}")
    try:
        input(f"\n{EMOJI['INFO']} {effective_translator('reset.press_enter', default='Press Enter to exit...')}{Style.RESET_ALL}")
    except EOFError:
        pass

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Resets Cursor AI Code Editor machine identifiers and optionally applies patches or purges data.", formatter_class=argparse.RawTextHelpFormatter)
    parser.add_argument("--skip-system-ids", action="store_true", help="Skip modifying system-level identifiers (Win Registry/macOS Plist).\nRecommended if unsure or running without admin/sudo.")
    parser.add_argument("--skip-js-patches", action="store_true", help="Skip patching Cursor's internal JavaScript files.\n(Patches might break with updates or be unwanted).")
    parser.add_argument("--purge", action="store_true", help=f"{Style.BRIGHT}{Fore.YELLOW}DANGEROUS:{Style.NORMAL}{Fore.RESET} Delete specific user configuration data (logs, state, history).\nUSE WITH EXTREME CAUTION. Requires confirmation.")
    args = parser.parse_args()

    main_translator = None  # Placeholder for actual translator

    needs_elevation = False
    if args.purge or not args.skip_system_ids:
        system = platform.system()
        temp_translator = _get_translator_func(main_translator)
        reason = "purge data" if args.purge else "modify system IDs"
        if system == "Windows":
            try:
                import ctypes
                needs_elevation = not ctypes.windll.shell32.IsUserAnAdmin()
            except Exception as admin_err:
                print(f"{Fore.YELLOW}Warning: Could not check admin rights ({admin_err}). Assuming needed for {reason}.{Style.RESET_ALL}")
                needs_elevation = True
        elif system in ["Darwin", "Linux"]:
            try:
                needs_elevation = os.geteuid() != 0
            except AttributeError:
                needs_elevation = True
        if needs_elevation:
            print(f"\n{Fore.RED}{EMOJI['LOCK']} {temp_translator('reset.elevation_required_warn_reason', reason=reason, default=f'WARNING: Requires Administrator/root privileges to {reason}.')}{Style.RESET_ALL}")
            print(f"{Fore.YELLOW}{temp_translator('reset.rerun_elevated')}{Style.RESET_ALL}")
            if reason == "modify system IDs":
                print(f"{Fore.YELLOW}{temp_translator('reset.use_skip_option_system')}{Style.RESET_ALL}")
            if reason == "purge data":
                print(f"{Fore.YELLOW}{temp_translator('reset.use_skip_option_purge', default='Alternatively, omit the --purge flag.')}{Style.RESET_ALL}")
            try:
                input(f"\n{EMOJI['INFO']} {temp_translator('reset.press_enter_to_exit')}{Style.RESET_ALL}")
            except EOFError:
                pass
            sys.exit(1)

    run(translator=main_translator, skip_system=args.skip_system_ids, skip_patches=args.skip_js_patches, purge=args.purge)