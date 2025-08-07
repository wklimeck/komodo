import sys
import os
import shutil
import platform
import json
import urllib.request

def load_version():
	version = ""
	for arg in sys.argv:
		if arg.count("--version") > 0:
			version = arg.split("=")[1]
	if len(version) == 0:
		version = load_latest_version()
	return version

def load_latest_version():
	return json.load(urllib.request.urlopen("https://api.github.com/repos/moghtech/komodo/releases/latest"))["tag_name"]

def load_paths():
	home_dir = os.environ['HOME']
	# Checks if setup.py is passed --user arg
	user_install = sys.argv.count("--user") > 0
	if user_install:
		return [
			# binary dir
			f'{home_dir}/.local/bin',
      # config dir
	 		f'{home_dir}/.config/komodo',
		]
	else:
		return [
			# binary dir
			"/usr/local/bin",
      # config dir
	 		f'{home_dir}/.config/komodo',
		]

def copy_binary(bin_dir, version):
	# ensure bin_dir exists
	if not os.path.isdir(bin_dir):
		os.makedirs(bin_dir)

	# delete binary if it already exists
	bin_path = f'{bin_dir}/km'
	if os.path.isfile(bin_path):
		os.remove(bin_path)

	km_bin = "km-x86_64"
	arch = platform.machine().lower()
	if arch == "aarch64" or arch == "arm64":
		print("aarch64 detected")
		km_bin = "km-aarch64"
	else:
		print("using x86_64 binary")

	# download the binary to bin path
	print(os.popen(f'curl -sSL https://github.com/moghtech/komodo/releases/download/{version}/{km_bin} > {bin_path}').read())

	# add executable permissions
	os.popen(f'chmod +x {bin_path}')

def copy_config(config_dir):
	config_file = f'{config_dir}/komodo.cli.toml'

	# early return if config file already exists
	if os.path.isfile(config_file):
		print("config already exists, skipping...")
		return
	
	print(f'creating config at {config_file}')

	# ensure config dir exists
	if not os.path.isdir(config_dir):
		os.makedirs(config_dir)

	print(os.popen(f'curl -sSL https://raw.githubusercontent.com/moghtech/komodo/main/config/komodo.cli.toml > {config_file}').read())
	
def main():
	print("======================")
	print(" KOMODO CLI INSTALLER ")
	print("======================")

	version = load_version()
	[bin_dir, config_dir] = load_paths()
 
	print(f'version: {version}')
	print(f'install to: {bin_dir}/km')
	print(f'config path: {config_dir}/komodo.cli.toml')

	copy_binary(bin_dir, version)
	copy_config(config_dir)

	print("Finished komodo-cli setup. Try running 'km --help'.\n")

main()
