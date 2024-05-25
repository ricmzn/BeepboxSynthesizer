extends Container

@onready var player: AudioStreamPlayer = %AudioStreamPlayer

@onready var buttons: Container = $ChannelButtons
@onready var buttonTemplate: Button = $ChannelButtons/ToggleChannel

@onready var playButton: Button = $PlayPause
@onready var bpmLabel: Label = $BPMLabel
@onready var bpmSlider: Slider = $BPMSlider
@onready var volumeLabel: Label = $VolumeLabel
@onready var volumeSlider: Slider = $VolumeSlider
@onready var progress: ProgressBar = $ProgressBar

var playback: AudioStreamPlaybackBeepBox

func _ready():
	buttonTemplate.get_parent().remove_child(buttonTemplate)
	bpmSlider.value_changed.connect(set_bpm)
	volumeSlider.value_changed.connect(set_volume)
	playback = player.get_stream_playback()
	update()

func update():
	for button in buttons.get_children():
		button.queue_free()

	if playback != null and playback.eval("typeof(player) !== 'undefined' && player.song != null"):
		playButton.disabled = false
		var channels = playback.eval("player.song.channels.length")
		for i in range(channels):
			var button: Button = buttonTemplate.duplicate()
			var instrumentName = playback.eval(
				"beepbox.Config.instrumentTypeNames[player.song.channels[%d].instruments[0].type]" % i)
			button.active = playback.eval("player.song.channels[%d].muted === false" % i)
			button.description = instrumentName
			button.index = i
			button.player = player
			button.update_text()
			buttons.add_child(button)
			update_bpm()
	else:
		playButton.disabled = true

func _physics_process(delta):
	if playback != null and playback.eval("player.song != null") and playback.eval("player.song.channels.length > 0"):
		progress.value = playback.eval("player.playhead") / playback.eval("player.song.barCount") * 100
	else:
		progress.value = 0

func update_bpm():
	var tempo: int = playback.eval("player.song.tempo")
	bpmLabel.text = "Beats per Minute: %d" % tempo
	bpmSlider.value = tempo

func set_bpm(tempo: int):
	if playback != null:
		playback.eval("player.song.tempo = %d" % tempo)
		update_bpm()

func set_volume(percent: int):
	volumeLabel.text = "Volume: %d%%" % int(percent)
	AudioServer.set_bus_volume_db(0, linear_to_db(percent / 100.0))
