extends Container

@onready var synth: Synthesizer = %Synthesizer

@onready var buttons: Container = $ChannelButtons
@onready var buttonTemplate: Button = $ChannelButtons/ToggleChannel

@onready var playButton: Button = $PlayPause
@onready var bpmLabel: Label = $BPMLabel
@onready var bpmSlider: Slider = $BPMSlider
@onready var progress: ProgressBar = $ProgressBar

func _ready():
	buttonTemplate.get_parent().remove_child(buttonTemplate)
	bpmSlider.connect("value_changed", set_bpm)
	update()

func update():
	for button in buttons.get_children():
		button.queue_free()

	if synth != null and synth.eval("typeof(synth) !== 'undefined' && synth.song != null"):
		playButton.synth = synth
		playButton.disabled = false
		var channels = synth.eval("synth.song.channels.length")
		for i in range(channels):
			var button: Button = buttonTemplate.duplicate()
			var instrumentName = synth.eval(
				"beepbox.Config.instrumentTypeNames[synth.song.channels[%d].instruments[0].type]" % i)
			button.active = synth.eval("synth.song.channels[%d].muted === false" % i)
			button.description = instrumentName
			button.index = i
			button.synth = synth
			button.update_text()
			buttons.add_child(button)
			update_bpm()
	else:
		playButton.disabled = true

func _physics_process(delta):
	if synth != null and synth.eval("synth.song != null"):
		progress.value = synth.eval("synth.playhead") / synth.eval("synth.song.barCount") * 100

func update_bpm():
	var tempo: int = synth.eval("synth.song.tempo")
	bpmLabel.text = "Beats per Minute: %d" % tempo
	bpmSlider.value = tempo

func set_bpm(tempo: int):
	if synth != null:
		synth.eval("synth.song.tempo = %d" % tempo)
		update_bpm()
