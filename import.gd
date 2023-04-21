extends Button

@onready var fileDialog := $"/root/Main/FileDialog"
@onready var player := $"/root/Main/CenterContainer/MediaPlayer"
@onready var synth: Synthesizer = $"/root/Main/Synthesizer"
@export var synth_volume := -25

func _on_Import_pressed():
	fileDialog.popup()

func _on_FileDialog_file_selected(path: String):
	synth.import(path)
	player.update()
