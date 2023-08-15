// see: https://github.com/jbaublitz/neli/blob/v0.7.0-rc2/examples/nl80211.rs
// and: `/usr/include/linux/nl80211.h`

#[neli::neli_enum(serialized_type = "u8")]
pub enum Nl80211Command {
    /// unspecified command to catch errors
    Unspec = 0,
    /// request information about a wiphy or dump request
    /// to get a list of all present wiphys.
    GetWiphy = 1,
    /// set wiphy parameters, needs %NL80211_ATTR_WIPHY or
    /// %NL80211_ATTR_IFINDEX; can be used to set %NL80211_ATTR_WIPHY_NAME,
    /// %NL80211_ATTR_WIPHY_TXQ_PARAMS, %NL80211_ATTR_WIPHY_FREQ (and the
    /// attributes determining the channel width; this is used for setting
    /// monitor mode channel),  %NL80211_ATTR_WIPHY_RETRY_SHORT,
    /// %NL80211_ATTR_WIPHY_RETRY_LONG, %NL80211_ATTR_WIPHY_FRAG_THRESHOLD,
    /// and/or %NL80211_ATTR_WIPHY_RTS_THRESHOLD.
    /// However, for setting the channel, see %NL80211_CMD_SET_CHANNEL
    /// instead, the support here is for backward compatibility only.
    SetWiphy = 2,
    /// Newly created wiphy, response to get request
    /// or rename notification. Has attributes %NL80211_ATTR_WIPHY and
    /// %NL80211_ATTR_WIPHY_NAME.
    NewWiphy = 3,
    /// Wiphy deleted. Has attributes
    /// %NL80211_ATTR_WIPHY and %NL80211_ATTR_WIPHY_NAME.
    DelWiphy = 4,

    /// Request an interface's configuration;
    /// either a dump request for all interfaces or a specific get with a
    /// single %NL80211_ATTR_IFINDEX is supported.
    GetInterface = 5,
    /// Set type of a virtual interface, requires
    /// %NL80211_ATTR_IFINDEX and %NL80211_ATTR_IFTYPE.
    SetInterface = 6,
    /// Newly created virtual interface or response
    /// to %NL80211_CMD_GET_INTERFACE. Has %NL80211_ATTR_IFINDEX,
    /// %NL80211_ATTR_WIPHY and %NL80211_ATTR_IFTYPE attributes. Can also
    /// be sent from userspace to request creation of a new virtual interface,
    /// then requires attributes %NL80211_ATTR_WIPHY, %NL80211_ATTR_IFTYPE and
    /// %NL80211_ATTR_IFNAME.
    NewInterface = 7,
    /// Virtual interface was deleted, has attributes
    /// %NL80211_ATTR_IFINDEX and %NL80211_ATTR_WIPHY. Can also be sent from
    /// userspace to request deletion of a virtual interface, then requires
    /// attribute %NL80211_ATTR_IFINDEX.
    DelInterface = 8,

    /// Get sequence counter information for a key specified
    /// by %NL80211_ATTR_KEY_IDX and/or %NL80211_ATTR_MAC.
    GetKey = 9,
    /// Set key attributes %NL80211_ATTR_KEY_DEFAULT,
    /// %NL80211_ATTR_KEY_DEFAULT_MGMT, or %NL80211_ATTR_KEY_THRESHOLD.
    SetKey = 10,
    /// add a key with given %NL80211_ATTR_KEY_DATA,
    /// %NL80211_ATTR_KEY_IDX, %NL80211_ATTR_MAC, %NL80211_ATTR_KEY_CIPHER,
    /// and %NL80211_ATTR_KEY_SEQ attributes.
    NewKey = 11,
    /// delete a key identified by %NL80211_ATTR_KEY_IDX
    /// or %NL80211_ATTR_MAC.
    DelKey = 12,

    /// (not used)
    GetBeacon = 13,
    /// change the beacon on an access point interface
    /// using the %NL80211_ATTR_BEACON_HEAD and %NL80211_ATTR_BEACON_TAIL
    /// attributes. For drivers that generate the beacon and probe responses
    /// internally, the following attributes must be provided: %NL80211_ATTR_IE,
    /// %NL80211_ATTR_IE_PROBE_RESP and %NL80211_ATTR_IE_ASSOC_RESP.
    SetBeacon = 14,
    /// Start AP operation on an AP interface, parameters
    /// are like for %NL80211_CMD_SET_BEACON, and additionally parameters that
    /// do not change are used, these include %NL80211_ATTR_BEACON_INTERVAL,
    /// %NL80211_ATTR_DTIM_PERIOD, %NL80211_ATTR_SSID,
    /// %NL80211_ATTR_HIDDEN_SSID, %NL80211_ATTR_CIPHERS_PAIRWISE,
    /// %NL80211_ATTR_CIPHER_GROUP, %NL80211_ATTR_WPA_VERSIONS,
    /// %NL80211_ATTR_AKM_SUITES, %NL80211_ATTR_PRIVACY,
    /// %NL80211_ATTR_AUTH_TYPE, %NL80211_ATTR_INACTIVITY_TIMEOUT,
    /// %NL80211_ATTR_ACL_POLICY and %NL80211_ATTR_MAC_ADDRS.
    /// The channel to use can be set on the interface or be given using the
    /// %NL80211_ATTR_WIPHY_FREQ and the attributes determining channel width.
    StartAp = 15,
    /// old alias for %NL80211_CMD_START_AP
    NewBeacon = 15,
    /// Stop AP operation on the given interface
    StopAp = 16,
    /// old alias for %NL80211_CMD_STOP_AP
    DelBeacon = 16,

    /// Get station attributes for station identified by
    /// %NL80211_ATTR_MAC on the interface identified by %NL80211_ATTR_IFINDEX.
    GetStation = 17,
    /// Set station attributes for station identified by
    /// %NL80211_ATTR_MAC on the interface identified by %NL80211_ATTR_IFINDEX.
    SetStation = 18,
    /// Add a station with given attributes to the
    /// the interface identified by %NL80211_ATTR_IFINDEX.
    NewStation = 19,
    /// Remove a station identified by %NL80211_ATTR_MAC
    /// or, if no MAC address given, all stations, on the interface identified
    /// by %NL80211_ATTR_IFINDEX. %NL80211_ATTR_MGMT_SUBTYPE and
    /// %NL80211_ATTR_REASON_CODE can optionally be used to specify which type
    /// of disconnection indication should be sent to the station
    /// (Deauthentication or Disassociation frame and reason code for that
    /// frame).
    DelStation = 20,

    /// Get mesh path attributes for mesh path to
    /// destination %NL80211_ATTR_MAC on the interface identified by
    /// %NL80211_ATTR_IFINDEX.
    GetMpath = 21,
    /// Set mesh path attributes for mesh path to
    /// destination %NL80211_ATTR_MAC on the interface identified by
    /// %NL80211_ATTR_IFINDEX.
    SetMpath = 22,
    /// Create a new mesh path for the destination given by
    /// %NL80211_ATTR_MAC via %NL80211_ATTR_MPATH_NEXT_HOP.
    NewMpath = 23,
    /// Delete a mesh path to the destination given by
    /// %NL80211_ATTR_MAC.
    DelMpath = 24,

    /// Set BSS attributes for BSS identified by
    /// %NL80211_ATTR_IFINDEX.
    SetBss = 25,

    /// Set current regulatory domain. CRDA sends this command
    /// after being queried by the kernel. CRDA replies by sending a regulatory
    /// domain structure which consists of %NL80211_ATTR_REG_ALPHA set to our
    /// current alpha2 if it found a match. It also provides
    /// NL80211_ATTR_REG_RULE_FLAGS, and a set of regulatory rules. Each
    /// regulatory rule is a nested set of attributes  given by
    /// %NL80211_ATTR_REG_RULE_FREQ_[START|END] and
    /// %NL80211_ATTR_FREQ_RANGE_MAX_BW with an attached power rule given by
    /// %NL80211_ATTR_REG_RULE_POWER_MAX_ANT_GAIN and
    /// %NL80211_ATTR_REG_RULE_POWER_MAX_EIRP.
    SetReg = 26,
    /// ask the wireless core to set the regulatory domain
    /// to the specified ISO/IEC 3166-1 alpha2 country code. The core will
    /// store this as a valid request and then query userspace for it.
    ReqSetReg = 27,

    /// Get mesh networking properties for the
    /// interface identified by %NL80211_ATTR_IFINDEX
    GetMeshConfig = 28,
    /// Set mesh networking properties for the
    /// interface identified by %NL80211_ATTR_IFINDEX
    SetMeshConfig = 29,

    /// Set extra IEs for management frames. The
    /// interface is identified with %NL80211_ATTR_IFINDEX and the management
    /// frame subtype with %NL80211_ATTR_MGMT_SUBTYPE. The extra IE data to be
    /// added to the end of the specified management frame is specified with
    /// %NL80211_ATTR_IE. If the command succeeds, the requested data will be
    /// added to all specified management frames generated by
    /// kernel/firmware/driver.
    /// Note: This command has been removed and it is only reserved at this
    /// point to avoid re-using existing command number. The functionality this
    /// command was planned for has been provided with cleaner design with the
    /// option to specify additional IEs in NL80211_CMD_TRIGGER_SCAN,
    /// NL80211_CMD_AUTHENTICATE, NL80211_CMD_ASSOCIATE,
    /// NL80211_CMD_DEAUTHENTICATE, and NL80211_CMD_DISASSOCIATE.
    SetMgmtExtraIe = 30,

    /// ask the wireless core to send us its currently set
    /// regulatory domain. If %NL80211_ATTR_WIPHY is specified and the device
    /// has a private regulatory domain, it will be returned. Otherwise, the
    /// global regdomain will be returned.
    /// A device will have a private regulatory domain if it uses the
    /// regulatory_hint() API. Even when a private regdomain is used the channel
    /// information will still be mended according to further hints from
    /// the regulatory core to help with compliance. A dump version of this API
    /// is now available which will returns the global regdomain as well as
    /// all private regdomains of present wiphys (for those that have it).
    /// If a wiphy is self-managed (%NL80211_ATTR_WIPHY_SELF_MANAGED_REG), then
    /// its private regdomain is the only valid one for it. The regulatory
    /// core is not used to help with compliance in this case.
    GetReg = 31,

    /// get scan results; can dump
    GetScan = 32,
    /// trigger a new scan with the given parameters
    /// %NL80211_ATTR_TX_NO_CCK_RATE is used to decide whether to send the
    /// probe requests at CCK rate or not. %NL80211_ATTR_MAC can be used to
    /// specify a BSSID to scan for; if not included, the wildcard BSSID will
    /// be used.
    TriggerScan = 33,
    /// scan notification (as a reply to
    /// NL80211_CMD_GET_SCAN and on the "scan" multicast group)
    NewScanResults = 34,
    /// scan was aborted, for unspecified reasons,
    /// partial scan results may be available
    ScanAborted = 35,

    /// indicates to userspace the regulatory domain
    /// has been changed and provides details of the request information
    /// that caused the change such as who initiated the regulatory request
    /// (%NL80211_ATTR_REG_INITIATOR), the wiphy_idx
    /// (%NL80211_ATTR_REG_ALPHA2) on which the request was made from if
    /// the initiator was %NL80211_REGDOM_SET_BY_COUNTRY_IE or
    /// %NL80211_REGDOM_SET_BY_DRIVER, the type of regulatory domain
    /// set (%NL80211_ATTR_REG_TYPE), if the type of regulatory domain is
    /// %NL80211_REG_TYPE_COUNTRY the alpha2 to which we have moved on
    /// to (%NL80211_ATTR_REG_ALPHA2).
    RegChange = 36,
    /// authentication request and notification.
    /// This command is used both as a command (request to authenticate) and
    /// as an event on the "mlme" multicast group indicating completion of the
    /// authentication process.
    /// When used as a command, %NL80211_ATTR_IFINDEX is used to identify the
    /// interface. %NL80211_ATTR_MAC is used to specify PeerSTAAddress (and
    /// BSSID in case of station mode). %NL80211_ATTR_SSID is used to specify
    /// the SSID (mainly for association, but is included in authentication
    /// request, too, to help BSS selection. %NL80211_ATTR_WIPHY_FREQ is used
    /// to specify the frequence of the channel in MHz. %NL80211_ATTR_AUTH_TYPE
    /// is used to specify the authentication type. %NL80211_ATTR_IE is used to
    /// define IEs (VendorSpecificInfo, but also including RSN IE and FT IEs)
    /// to be added to the frame.
    /// When used as an event, this reports reception of an Authentication
    /// frame in station and IBSS modes when the local MLME processed the
    /// frame, i.e., it was for the local STA and was received in correct
    /// state. This is similar to MLME-AUTHENTICATE.confirm primitive in the
    /// MLME SAP interface (kernel providing MLME, userspace SME). The
    /// included %NL80211_ATTR_FRAME attribute contains the management frame
    /// (including both the header and frame body, but not FCS). This event is
    /// also used to indicate if the authentication attempt timed out. In that
    /// case the %NL80211_ATTR_FRAME attribute is replaced with a
    /// %NL80211_ATTR_TIMED_OUT flag (and %NL80211_ATTR_MAC to indicate which
    /// pending authentication timed out).
    Authenticate = 37,
    /// association request and notification; like
    /// NL80211_CMD_AUTHENTICATE but for Association and Reassociation
    /// (similar to MLME-ASSOCIATE.request, MLME-REASSOCIATE.request,
    /// MLME-ASSOCIATE.confirm or MLME-REASSOCIATE.confirm primitives). The
    /// %NL80211_ATTR_PREV_BSSID attribute is used to specify whether the
    /// request is for the initial association to an ESS (that attribute not
    /// included) or for reassociation within the ESS (that attribute is
    /// included).
    Associate = 38,
    /// deauthentication request and notification; like
    /// NL80211_CMD_AUTHENTICATE but for Deauthentication frames (similar to
    /// MLME-DEAUTHENTICATION.request and MLME-DEAUTHENTICATE.indication
    /// primitives).
    Deauthenticate = 39,
    /// disassociation request and notification; like
    /// NL80211_CMD_AUTHENTICATE but for Disassociation frames (similar to
    /// MLME-DISASSOCIATE.request and MLME-DISASSOCIATE.indication primitives).
    Disassociate = 40,

    /// notification of a locally detected Michael
    /// MIC (part of TKIP) failure; sent on the "mlme" multicast group; the
    /// event includes %NL80211_ATTR_MAC to describe the source MAC address of
    /// the frame with invalid MIC, %NL80211_ATTR_KEY_TYPE to show the key
    /// type, %NL80211_ATTR_KEY_IDX to indicate the key identifier, and
    /// %NL80211_ATTR_KEY_SEQ to indicate the TSC value of the frame; this
    /// event matches with MLME-MICHAELMICFAILURE.indication() primitive
    MichaelMicFailure = 41,

    /// indicates to userspace that an AP beacon
    /// has been found while world roaming thus enabling active scan or
    /// any mode of operation that initiates TX (beacons) on a channel
    /// where we would not have been able to do either before. As an example
    /// if you are world roaming (regulatory domain set to world or if your
    /// driver is using a custom world roaming regulatory domain) and while
    /// doing a passive scan on the 5 GHz band you find an AP there (if not
    /// on a DFS channel) you will now be able to actively scan for that AP
    /// or use AP mode on your card on that same channel. Note that this will
    /// never be used for channels 1-11 on the 2 GHz band as they are always
    /// enabled world wide. This beacon hint is only sent if your device had
    /// either disabled active scanning or beaconing on a channel. We send to
    /// userspace the wiphy on which we removed a restriction from
    /// (%NL80211_ATTR_WIPHY) and the channel on which this occurred
    /// before (%NL80211_ATTR_FREQ_BEFORE) and after (%NL80211_ATTR_FREQ_AFTER)
    /// the beacon hint was processed.
    RegBeaconHint = 42,

    /// Join a new IBSS -- given at least an SSID and a
    /// FREQ attribute (for the initial frequency if no peer can be found)
    /// and optionally a MAC (as BSSID) and FREQ_FIXED attribute if those
    /// should be fixed rather than automatically determined. Can only be
    /// executed on a network interface that is UP, and fixed BSSID/FREQ
    /// may be rejected. Another optional parameter is the beacon interval,
    /// given in the %NL80211_ATTR_BEACON_INTERVAL attribute, which if not
    /// given defaults to 100 TU (102.4ms).
    JoinIbss = 43,
    /// Leave the IBSS -- no special arguments, the IBSS is
    /// determined by the network interface.
    LeaveIbss = 44,

    /// testmode command, takes a wiphy (or ifindex) attribute
    /// to identify the device, and the TESTDATA blob attribute to pass through
    /// to the driver.
    Testmode = 45,

    /// connection request and notification; this command
    /// requests to connect to a specified network but without separating
    /// auth and assoc steps. For this, you need to specify the SSID in a
    /// %NL80211_ATTR_SSID attribute, and can optionally specify the association
    /// IEs in %NL80211_ATTR_IE, %NL80211_ATTR_AUTH_TYPE, %NL80211_ATTR_USE_MFP,
    /// %NL80211_ATTR_MAC, %NL80211_ATTR_WIPHY_FREQ, %NL80211_ATTR_CONTROL_PORT,
    /// %NL80211_ATTR_CONTROL_PORT_ETHERTYPE,
    /// %NL80211_ATTR_CONTROL_PORT_NO_ENCRYPT, %NL80211_ATTR_MAC_HINT, and
    /// %NL80211_ATTR_WIPHY_FREQ_HINT.
    /// If included, %NL80211_ATTR_MAC and %NL80211_ATTR_WIPHY_FREQ are
    /// restrictions on BSS selection, i.e., they effectively prevent roaming
    /// within the ESS. %NL80211_ATTR_MAC_HINT and %NL80211_ATTR_WIPHY_FREQ_HINT
    /// can be included to provide a recommendation of the initial BSS while
    /// allowing the driver to roam to other BSSes within the ESS and also to
    /// ignore this recommendation if the indicated BSS is not ideal. Only one
    /// set of BSSID,frequency parameters is used (i.e., either the enforcing
    /// %NL80211_ATTR_MAC,%NL80211_ATTR_WIPHY_FREQ or the less strict
    /// %NL80211_ATTR_MAC_HINT and %NL80211_ATTR_WIPHY_FREQ_HINT).
    /// %NL80211_ATTR_PREV_BSSID can be used to request a reassociation within
    /// the ESS in case the device is already associated and an association with
    /// a different BSS is desired.
    /// Background scan period can optionally be
    /// specified in %NL80211_ATTR_BG_SCAN_PERIOD,
    /// if not specified default background scan configuration
    /// in driver is used and if period value is 0, bg scan will be disabled.
    /// This attribute is ignored if driver does not support roam scan.
    /// It is also sent as an event, with the BSSID and response IEs when the
    /// connection is established or failed to be established. This can be
    /// determined by the %NL80211_ATTR_STATUS_CODE attribute (0 = success,
    /// non-zero = failure). If %NL80211_ATTR_TIMED_OUT is included in the
    /// event, the connection attempt failed due to not being able to initiate
    /// authentication/association or not receiving a response from the AP.
    /// Non-zero %NL80211_ATTR_STATUS_CODE value is indicated in that case as
    /// well to remain backwards compatible.
    Connect = 46,
    /// request that the card roam (currently not implemented),
    /// sent as an event when the card/driver roamed by itself.
    Roam = 47,
    /// drop a given connection; also used to notify
    /// userspace that a connection was dropped by the AP or due to other
    /// reasons, for this the %NL80211_ATTR_DISCONNECTED_BY_AP and
    /// %NL80211_ATTR_REASON_CODE attributes are used.
    Disconnect = 48,
    /// Set a wiphy's netns. Note that all devices
    /// associated with this wiphy must be down and will follow.
    SetWiphyNetns = 49,

    /// get survey resuls, e.g. channel occupation
    /// or noise level
    GetSurvey = 50,
    /// survey data notification (as a reply to
    /// NL80211_CMD_GET_SURVEY and on the "scan" multicast group)
    NewSurveyResults = 51,
    /// Add a PMKSA cache entry, using %NL80211_ATTR_MAC
    /// (for the BSSID) and %NL80211_ATTR_PMKID.
    SetPmksa = 52,
    /// Delete a PMKSA cache entry, using %NL80211_ATTR_MAC
    /// (for the BSSID) and %NL80211_ATTR_PMKID.
    DelPmksa = 53,
    /// Flush all PMKSA cache entries.
    FlushPmksa = 54,

    /// Request to remain awake on the specified
    /// channel for the specified amount of time. This can be used to do
    /// off-channel operations like transmit a Public Action frame and wait for
    /// a response while being associated to an AP on another channel.
    /// %NL80211_ATTR_IFINDEX is used to specify which interface (and thus
    /// radio) is used. %NL80211_ATTR_WIPHY_FREQ is used to specify the
    /// frequency for the operation.
    /// %NL80211_ATTR_DURATION is used to specify the duration in milliseconds
    /// to remain on the channel. This command is also used as an event to
    /// notify when the requested duration starts (it may take a while for the
    /// driver to schedule this time due to other concurrent needs for the
    /// radio).
    /// When called, this operation returns a cookie (%NL80211_ATTR_COOKIE)
    /// that will be included with any events pertaining to this request;
    /// the cookie is also used to cancel the request.
    RemainOnChannel = 55,
    /// This command can be used to cancel a
    /// pending remain-on-channel duration if the desired operation has been
    /// completed prior to expiration of the originally requested duration.
    /// %NL80211_ATTR_WIPHY or %NL80211_ATTR_IFINDEX is used to specify the
    /// radio. The %NL80211_ATTR_COOKIE attribute must be given as well to
    /// uniquely identify the request.
    /// This command is also used as an event to notify when a requested
    /// remain-on-channel duration has expired.
    CancelRemainOnChannel = 56,

    /// Set the mask of rates to be used in TX
    /// rate selection. %NL80211_ATTR_IFINDEX is used to specify the interface
    /// and @NL80211_ATTR_TX_RATES the set of allowed rates.
    SetTxBitrateMask = 57,

    /// Register for receiving certain mgmt frames
    /// (via @NL80211_CMD_FRAME) for processing in userspace. This command
    /// requires an interface index, a frame type attribute (optional for
    /// backward compatibility reasons, if not given assumes action frames)
    /// and a match attribute containing the first few bytes of the frame
    /// that should match, e.g. a single byte for only a category match or
    /// four bytes for vendor frames including the OUI. The registration
    /// cannot be dropped, but is removed automatically when the netlink
    /// socket is closed. Multiple registrations can be made.
    RegisterFrame = 58,
    /// Alias for @NL80211_CMD_REGISTER_FRAME for
    /// backward compatibility
    RegisterAction = 58,
    /// Management frame TX request and RX notification. This
    /// command is used both as a request to transmit a management frame and
    /// as an event indicating reception of a frame that was not processed in
    /// kernel code, but is for us (i.e., which may need to be processed in a
    /// user space application). %NL80211_ATTR_FRAME is used to specify the
    /// frame contents (including header). %NL80211_ATTR_WIPHY_FREQ is used
    /// to indicate on which channel the frame is to be transmitted or was
    /// received. If this channel is not the current channel (remain-on-channel
    /// or the operational channel) the device will switch to the given channel
    /// and transmit the frame, optionally waiting for a response for the time
    /// specified using %NL80211_ATTR_DURATION. When called, this operation
    /// returns a cookie (%NL80211_ATTR_COOKIE) that will be included with the
    /// TX status event pertaining to the TX request.
    /// %NL80211_ATTR_TX_NO_CCK_RATE is used to decide whether to send the
    /// management frames at CCK rate or not in 2GHz band.
    /// %NL80211_ATTR_CSA_C_OFFSETS_TX is an array of offsets to CSA
    /// counters which will be updated to the current value. This attribute
    /// is used during CSA period.
    Frame = 59,
    /// Alias for @NL80211_CMD_FRAME for backward compatibility.
    /// @NL80211_CMD_FRAME_TX_STATUS: Report TX status of a management frame
    /// transmitted with %NL80211_CMD_FRAME. %NL80211_ATTR_COOKIE identifies
    /// the TX command and %NL80211_ATTR_FRAME includes the contents of the
    /// frame. %NL80211_ATTR_ACK flag is included if the recipient acknowledged
    /// the frame.
    Action = 59,
    FrameTxStatus = 60,
    /// Alias for @NL80211_CMD_FRAME_TX_STATUS for
    /// backward compatibility.
    ActionTxStatus = 60,
    /// Set powersave, using %NL80211_ATTR_PS_STATE
    SetPowerSave = 61,
    /// Get powersave status in %NL80211_ATTR_PS_STATE
    GetPowerSave = 62,
    /// Connection quality monitor configuration. This command
    /// is used to configure connection quality monitoring notification trigger
    /// levels.
    SetCqm = 63,
    /// Connection quality monitor notification. This
    /// command is used as an event to indicate the that a trigger level was
    /// reached.
    NotifyCqm = 64,
    /// Set the channel (using %NL80211_ATTR_WIPHY_FREQ
    /// and the attributes determining channel width) the given interface
    /// (identifed by %NL80211_ATTR_IFINDEX) shall operate on.
    /// In case multiple channels are supported by the device, the mechanism
    /// with which it switches channels is implementation-defined.
    /// When a monitor interface is given, it can only switch channel while
    /// no other interfaces are operating to avoid disturbing the operation
    /// of any other interfaces, and other interfaces will again take
    /// precedence when they are used.
    SetChannel = 65,

    /// Set the MAC address of the peer on a WDS interface.
    SetWdsPeer = 66,

    /// When an off-channel TX was requested, this
    /// command may be used with the corresponding cookie to cancel the wait
    /// time if it is known that it is no longer necessary.
    FrameWaitCancel = 67,

    /// Join a mesh. The mesh ID must be given, and initial
    /// mesh config parameters may be given.
    JoinMesh = 68,
    /// Leave the mesh network -- no special arguments, the
    /// network is determined by the network interface.
    LeaveMesh = 69,

    /// Unprotected deauthentication frame
    /// notification. This event is used to indicate that an unprotected
    /// deauthentication frame was dropped when MFP is in use.
    UnprotDeauthenticate = 70,
    /// Unprotected disassociation frame
    /// notification. This event is used to indicate that an unprotected
    /// disassociation frame was dropped when MFP is in use.
    UnprotDisassociate = 71,

    /// Notification on the reception of a
    /// beacon or probe response from a compatible mesh peer.  This is only
    /// sent while no station information (sta_info) exists for the new peer
    /// candidate and when @NL80211_MESH_SETUP_USERSPACE_AUTH,
    /// @NL80211_MESH_SETUP_USERSPACE_AMPE, or
    /// @NL80211_MESH_SETUP_USERSPACE_MPM is set.  On reception of this
    /// notification, userspace may decide to create a new station
    /// (@NL80211_CMD_NEW_STATION).  To stop this notification from
    /// reoccurring, the userspace authentication daemon may want to create the
    /// new station with the AUTHENTICATED flag unset and maybe change it later
    /// depending on the authentication result.
    NewPeerCandidate = 72,

    /// get Wake-on-Wireless-LAN (WoWLAN) settings.
    GetWowlan = 73,
    /// set Wake-on-Wireless-LAN (WoWLAN) settings.
    /// Since wireless is more complex than wired ethernet, it supports
    /// various triggers. These triggers can be configured through this
    /// command with the %NL80211_ATTR_WOWLAN_TRIGGERS attribute. For
    /// more background information, see
    /// http://wireless.kernel.org/en/users/Documentation/WoWLAN.
    /// The @NL80211_CMD_SET_WOWLAN command can also be used as a notification
    /// from the driver reporting the wakeup reason. In this case, the
    /// @NL80211_ATTR_WOWLAN_TRIGGERS attribute will contain the reason
    /// for the wakeup, if it was caused by wireless. If it is not present
    /// in the wakeup notification, the wireless device didn't cause the
    /// wakeup but reports that it was woken up.
    SetWowlan = 74,

    /// start a scheduled scan at certain
    /// intervals and certain number of cycles, as specified by
    /// %NL80211_ATTR_SCHED_SCAN_PLANS. If %NL80211_ATTR_SCHED_SCAN_PLANS is
    /// not specified and only %NL80211_ATTR_SCHED_SCAN_INTERVAL is specified,
    /// scheduled scan will run in an infinite loop with the specified interval.
    /// These attributes are mutually exculsive,
    /// i.e. NL80211_ATTR_SCHED_SCAN_INTERVAL must not be passed if
    /// NL80211_ATTR_SCHED_SCAN_PLANS is defined.
    /// If for some reason scheduled scan is aborted by the driver, all scan
    /// plans are canceled (including scan plans that did not start yet).
    /// Like with normal scans, if SSIDs (%NL80211_ATTR_SCAN_SSIDS)
    /// are passed, they are used in the probe requests.  For
    /// broadcast, a broadcast SSID must be passed (ie. an empty
    /// string).  If no SSID is passed, no probe requests are sent and
    /// a passive scan is performed.  %NL80211_ATTR_SCAN_FREQUENCIES,
    /// if passed, define which channels should be scanned; if not
    /// passed, all channels allowed for the current regulatory domain
    /// are used.  Extra IEs can also be passed from the userspace by
    /// using the %NL80211_ATTR_IE attribute.  The first cycle of the
    /// scheduled scan can be delayed by %NL80211_ATTR_SCHED_SCAN_DELAY
    /// is supplied.
    StartSchedScan = 75,
    /// stop a scheduled scan. Returns -ENOENT if
    /// scheduled scan is not running. The caller may assume that as soon
    /// as the call returns, it is safe to start a new scheduled scan again.
    StopSchedScan = 76,
    /// indicates that there are scheduled scan
    /// results available.
    SchedScanResults = 77,
    /// indicates that the scheduled scan has
    /// stopped.  The driver may issue this event at any time during a
    /// scheduled scan.  One reason for stopping the scan is if the hardware
    /// does not support starting an association or a normal scan while running
    /// a scheduled scan.  This event is also sent when the
    /// %NL80211_CMD_STOP_SCHED_SCAN command is received or when the interface
    /// is brought down while a scheduled scan was running.
    SchedScanStopped = 78,

    /// This command is used give the driver
    /// the necessary information for supporting GTK rekey offload. This
    /// feature is typically used during WoWLAN. The configuration data
    /// is contained in %NL80211_ATTR_REKEY_DATA (which is nested and
    /// contains the data in sub-attributes). After rekeying happened,
    /// this command may also be sent by the driver as an MLME event to
    /// inform userspace of the new replay counter.
    SetRekeyOffload = 79,

    /// This is used as an event to inform userspace
    /// of PMKSA caching dandidates
    PmksaCandidate = 80,

    /// Perform a high-level TDLS command (e.g. link setup).
    /// In addition, this can be used as an event to request userspace to take
    /// actions on TDLS links (set up a new link or tear down an existing one).
    /// In such events, %NL80211_ATTR_TDLS_OPERATION indicates the requested
    /// operation, %NL80211_ATTR_MAC contains the peer MAC address, and
    /// %NL80211_ATTR_REASON_CODE the reason code to be used (only with
    /// %NL80211_TDLS_TEARDOWN).
    TdlsOper = 81,
    /// Send a TDLS management frame. The
    /// %NL80211_ATTR_TDLS_ACTION attribute determines the type of frame to be
    /// sent. Public Action codes (802.11-2012 8.1.5.1) will be sent as
    /// 802.11 management frames, while TDLS action codes (802.11-2012
    /// 8.5.13.1) will be encapsulated and sent as data frames. The currently
    /// supported Public Action code is %WLAN_PUB_ACTION_TDLS_DISCOVER_RES
    /// and the currently supported TDLS actions codes are given in
    /// &enum ieee80211_tdls_actioncode.
    TdlsMgmt = 82,

    /// Used by an application controlling an AP
    /// (or GO) interface (i.e. hostapd) to ask for unexpected frames to
    /// implement sending deauth to stations that send unexpected class 3
    /// frames. Also used as the event sent by the kernel when such a frame
    /// is received.
    /// For the event, the %NL80211_ATTR_MAC attribute carries the TA and
    /// other attributes like the interface index are present.
    /// If used as the command it must have an interface index and you can
    /// only unsubscribe from the event by closing the socket. Subscription
    /// is also for %NL80211_CMD_UNEXPECTED_4ADDR_FRAME events.
    UnexpectedFrame = 83,

    /// Probe an associated station on an AP interface
    /// by sending a null data frame to it and reporting when the frame is
    /// acknowleged. This is used to allow timing out inactive clients. Uses
    /// %NL80211_ATTR_IFINDEX and %NL80211_ATTR_MAC. The command returns a
    /// direct reply with an %NL80211_ATTR_COOKIE that is later used to match
    /// up the event with the request. The event includes the same data and
    /// has %NL80211_ATTR_ACK set if the frame was ACKed.
    ProbeClient = 84,

    /// Register this socket to receive beacons from
    /// other BSSes when any interfaces are in AP mode. This helps implement
    /// OLBC handling in hostapd. Beacons are reported in %NL80211_CMD_FRAME
    /// messages. Note that per PHY only one application may register.
    RegisterBeacons = 85,

    /// Sent as an event indicating that the
    /// associated station identified by %NL80211_ATTR_MAC sent a 4addr frame
    /// and wasn't already in a 4-addr VLAN. The event will be sent similarly
    /// to the %NL80211_CMD_UNEXPECTED_FRAME event, to the same listener.
    Unexpected4AddrFrame = 86,

    /// sets a bitmap for the individual TIDs whether
    /// No Acknowledgement Policy should be applied.
    SetNoackMap = 87,

    /// An AP or GO may decide to switch channels
    /// independently of the userspace SME, send this event indicating
    /// %NL80211_ATTR_IFINDEX is now on %NL80211_ATTR_WIPHY_FREQ and the
    /// attributes determining channel width.  This indication may also be
    /// sent when a remotely-initiated switch (e.g., when a STA receives a CSA
    /// from the remote AP) is completed;
    ChSwitchNotify = 88,

    /// Start the given P2P Device, identified by
    /// its %NL80211_ATTR_WDEV identifier. It must have been created with
    /// %NL80211_CMD_NEW_INTERFACE previously. After it has been started, the
    /// P2P Device can be used for P2P operations, e.g. remain-on-channel and
    /// public action frame TX.
    StartP2PDevice = 89,
    /// Stop the given P2P Device, identified by
    /// its %NL80211_ATTR_WDEV identifier.
    StopP2PDevice = 90,

    /// connection request to an AP failed; used to
    /// notify userspace that AP has rejected the connection request from a
    /// station, due to particular reason. %NL80211_ATTR_CONN_FAILED_REASON
    /// is used for this.
    ConnFailed = 91,

    /// Change the rate used to send multicast frames
    /// for IBSS or MESH vif.
    SetMcastRate = 92,

    /// sets ACL for MAC address based access control.
    /// This is to be used with the drivers advertising the support of MAC
    /// address based access control. List of MAC addresses is passed in
    /// %NL80211_ATTR_MAC_ADDRS and ACL policy is passed in
    /// %NL80211_ATTR_ACL_POLICY. Driver will enable ACL with this list, if it
    /// is not already done. The new list will replace any existing list. Driver
    /// will clear its ACL when the list of MAC addresses passed is empty. This
    /// command is used in AP/P2P GO mode. Driver has to make sure to clear its
    /// ACL list during %NL80211_CMD_STOP_AP.
    SetMacAcl = 93,

    /// Start a Channel availability check (CAC). Once
    /// a radar is detected or the channel availability scan (CAC) has finished
    /// or was aborted, or a radar was detected, usermode will be notified with
    /// this event. This command is also used to notify userspace about radars
    /// while operating on this channel.
    /// %NL80211_ATTR_RADAR_EVENT is used to inform about the type of the
    /// event.
    RadarDetect = 94,

    /// Get global nl80211 protocol features,
    /// i.e. features for the nl80211 protocol rather than device features.
    /// Returns the features in the %NL80211_ATTR_PROTOCOL_FEATURES bitmap.
    GetProtocolFeatures = 95,

    /// Pass down the most up-to-date Fast Transition
    /// Information Element to the WLAN driver
    UpdateFtIes = 96,

    /// Send a Fast transition event from the WLAN driver
    /// to the supplicant. This will carry the target AP's MAC address along
    /// with the relevant Information Elements. This event is used to report
    /// received FT IEs (MDIE, FTIE, RSN IE, TIE, RICIE).
    FtEvent = 97,

    /// Indicates user-space will start running
    /// a critical protocol that needs more reliability in the connection to
    /// complete.
    CritProtocolStart = 98,
    /// Indicates the connection reliability can
    /// return back to normal.
    CritProtocolStop = 99,

    /// Get currently supported coalesce rules.
    GetCoalesce = 100,
    /// Configure coalesce rules or clear existing rules.
    SetCoalesce = 101,

    /// Perform a channel switch by announcing the
    /// the new channel information (Channel Switch Announcement - CSA)
    /// in the beacon for some time (as defined in the
    /// %NL80211_ATTR_CH_SWITCH_COUNT parameter) and then change to the
    /// new channel. Userspace provides the new channel information (using
    /// %NL80211_ATTR_WIPHY_FREQ and the attributes determining channel
    /// width). %NL80211_ATTR_CH_SWITCH_BLOCK_TX may be supplied to inform
    /// other station that transmission must be blocked until the channel
    /// switch is complete.
    ChannelSwitch = 102,

    /// Vendor-specified command/event. The command is specified
    /// by the %NL80211_ATTR_VENDOR_ID attribute and a sub-command in
    /// %NL80211_ATTR_VENDOR_SUBCMD. Parameter(s) can be transported in
    /// %NL80211_ATTR_VENDOR_DATA.
    /// For feature advertisement, the %NL80211_ATTR_VENDOR_DATA attribute is
    /// used in the wiphy data as a nested attribute containing descriptions
    /// (&struct nl80211_vendor_cmd_info) of the supported vendor commands.
    /// This may also be sent as an event with the same attributes.
    Vendor = 103,

    /// Set Interworking QoS mapping for IP DSCP values.
    /// The QoS mapping information is included in %NL80211_ATTR_QOS_MAP. If
    /// that attribute is not included, QoS mapping is disabled. Since this
    /// QoS mapping is relevant for IP packets, it is only valid during an
    /// association. This is cleared on disassociation and AP restart.
    SetQosMap = 104,

    /// Ask the kernel to add a traffic stream for the given
    /// %NL80211_ATTR_TSID and %NL80211_ATTR_MAC with %NL80211_ATTR_USER_PRIO
    /// and %NL80211_ATTR_ADMITTED_TIME parameters.
    /// Note that the action frame handshake with the AP shall be handled by
    /// userspace via the normal management RX/TX framework, this only sets
    /// up the TX TS in the driver/device.
    /// If the admitted time attribute is not added then the request just checks
    /// if a subsequent setup could be successful, the intent is to use this to
    /// avoid setting up a session with the AP when local restrictions would
    /// make that impossible. However, the subsequent "real" setup may still
    /// fail even if the check was successful.
    AddTxTs = 105,
    /// Remove an existing TS with the %NL80211_ATTR_TSID
    /// and %NL80211_ATTR_MAC parameters. It isn't necessary to call this
    /// before removing a station entry entirely, or before disassociating
    /// or similar, cleanup will happen in the driver/device in this case.
    DelTxTs = 106,

    /// Get mesh path attributes for mesh proxy path to
    /// destination %NL80211_ATTR_MAC on the interface identified by
    /// %NL80211_ATTR_IFINDEX.
    GetMpp = 107,

    /// Join the OCB network. The center frequency and
    /// bandwidth of a channel must be given.
    JoinOcb = 108,
    /// Leave the OCB network -- no special arguments, the
    /// network is determined by the network interface.
    LeaveOcb = 109,

    /// Notify that a channel switch
    /// has been started on an interface, regardless of the initiator
    /// (ie. whether it was requested from a remote device or
    /// initiated on our own).  It indicates that
    /// %NL80211_ATTR_IFINDEX will be on %NL80211_ATTR_WIPHY_FREQ
    /// after %NL80211_ATTR_CH_SWITCH_COUNT TBTT's.  The userspace may
    /// decide to react to this indication by requesting other
    /// interfaces to change channel as well.
    ChSwitchStartedNotify = 110,

    /// Start channel-switching with a TDLS peer,
    /// identified by the %NL80211_ATTR_MAC parameter. A target channel is
    /// provided via %NL80211_ATTR_WIPHY_FREQ and other attributes determining
    /// channel width/type. The target operating class is given via
    /// %NL80211_ATTR_OPER_CLASS.
    /// The driver is responsible for continually initiating channel-switching
    /// operations and returning to the base channel for communication with the
    /// AP.
    TdlsChannelSwitch = 111,
    /// Stop channel-switching with a TDLS
    /// peer given by %NL80211_ATTR_MAC. Both peers must be on the base channel
    /// when this command completes.
    TdlsCancelChannelSwitch = 112,

    /// Similar to %NL80211_CMD_REG_CHANGE, but used
    /// as an event to indicate changes for devices with wiphy-specific regdom
    /// management.
    WiphyRegChange = 113,

    /// Stop an ongoing scan. Returns -ENOENT if a scan is
    /// not running. The driver indicates the status of the scan through
    /// cfg80211_scan_done().
    AbortScan = 114,

    /// Start NAN operation, identified by its
    /// %NL80211_ATTR_WDEV interface. This interface must have been previously
    /// created with %NL80211_CMD_NEW_INTERFACE. After it has been started, the
    /// NAN interface will create or join a cluster. This command must have a
    /// valid %NL80211_ATTR_NAN_MASTER_PREF attribute and optional
    /// %NL80211_ATTR_NAN_DUAL attributes.
    /// After this command NAN functions can be added.
    StartNan = 115,
    /// Stop the NAN operation, identified by
    /// its %NL80211_ATTR_WDEV interface.
    StopNan = 116,
    /// Add a NAN function. The function is defined
    /// with %NL80211_ATTR_NAN_FUNC nested attribute. When called, this
    /// operation returns the strictly positive and unique instance id
    /// (%NL80211_ATTR_NAN_FUNC_INST_ID) and a cookie (%NL80211_ATTR_COOKIE)
    /// of the function upon success.
    /// Since instance ID's can be re-used, this cookie is the right
    /// way to identify the function. This will avoid races when a termination
    /// event is handled by the user space after it has already added a new
    /// function that got the same instance id from the kernel as the one
    /// which just terminated.
    /// This cookie may be used in NAN events even before the command
    /// returns, so userspace shouldn't process NAN events until it processes
    /// the response to this command.
    /// Look at %NL80211_ATTR_SOCKET_OWNER as well.
    AddNanFunction = 117,
    /// Delete a NAN function by cookie.
    /// This command is also used as a notification sent when a NAN function is
    /// terminated. This will contain a %NL80211_ATTR_NAN_FUNC_INST_ID
    /// and %NL80211_ATTR_COOKIE attributes.
    DelNanFunction = 118,
    /// Change current NAN configuration. NAN
    /// must be operational (%NL80211_CMD_START_NAN was executed).
    /// It must contain at least one of the following attributes:
    /// %NL80211_ATTR_NAN_MASTER_PREF, %NL80211_ATTR_NAN_DUAL.
    ChangeNanConfig = 119,
    /// Notification sent when a match is reported.
    /// This will contain a %NL80211_ATTR_NAN_MATCH nested attribute and
    /// %NL80211_ATTR_COOKIE.
    NanMatch = 120,

    SetMulticastToUnicast = 121,
    UpdateConnectParams = 122,
    SetPmk = 123,
    DelPmk = 124,
    PortAuthorized = 125,
    ReloadRegdb = 126,
    ExternalAuth = 127,
    StaOpmodeChanged = 128,
    ControlPortFrame = 129,
    GetFtmResponderStats = 130,
    PeerMeasurementStart = 131,
    PeerMeasurementResult = 132,
    PeerMeasurementComplete = 133,
    NotifyRadar = 134,
    UpdateOweInfo = 135,
    ProbeMeshLink = 136,
    SetTidConfig = 137,
    UnprotBeacon = 138,
    ControlPortFrameTxStatus = 139,
    SetSarSpecs = 140,
    ObssColorCollision = 141,
    ColorChangeRequest = 142,
    ColorChangeStarted = 143,
    ColorChangeAborted = 144,
    ColorChangeCompleted = 145,
    SetFilsAad = 146,
    AssocComeback = 147,
    AddLink = 148,
    RemoveLink = 149,
    AddLinkSta = 150,
    ModifyLinkSta = 151,
    RemoveLinkSta = 152,
    Max = 152,
}
impl neli::consts::genl::Cmd for Nl80211Command {}

#[neli::neli_enum(serialized_type = "u16")]
pub enum Nl80211Attribute {
    Unspec = 0,
    Wiphy = 1,
    WiphyName = 2,
    Ifindex = 3,
    Ifname = 4,
    Iftype = 5,
    Mac = 6,
    KeyData = 7,
    KeyIdx = 8,
    KeyCipher = 9,
    KeySeq = 10,
    KeyDefault = 11,
    BeaconInterval = 12,
    DtimPeriod = 13,
    BeaconHead = 14,
    BeaconTail = 15,
    StaAid = 16,
    StaFlags = 17,
    StaListenInterval = 18,
    StaSupportedRates = 19,
    StaVlan = 20,
    StaInfo = 21,
    WiphyBands = 22,
    MntrFlags = 23,
    MeshId = 24,
    StaPlinkAction = 25,
    MpathNextHop = 26,
    MpathInfo = 27,
    BssCtsProt = 28,
    BssShortPreamble = 29,
    BssShortSlotTime = 30,
    HtCapability = 31,
    SupportedIftypes = 32,
    RegAlpha2 = 33,
    RegRules = 34,
    MeshConfig = 35,
    BssBasicRates = 36,
    WiphyTxqParams = 37,
    WiphyFreq = 38,
    WiphyChannelType = 39,
    KeyDefaultMgmt = 40,
    MgmtSubtype = 41,
    Ie = 42,
    MaxNumScanSsids = 43,
    ScanFrequencies = 44,
    ScanSsids = 45,
    /// replaces old SCAN_GENERATION
    Generation = 46,
    Bss = 47,
    RegInitiator = 48,
    RegType = 49,
    SupportedCommands = 50,
    Frame = 51,
    Ssid = 52,
    AuthType = 53,
    ReasonCode = 54,
    KeyType = 55,
    MaxScanIeLen = 56,
    CipherSuites = 57,
    FreqBefore = 58,
    FreqAfter = 59,
    FreqFixed = 60,
    WiphyRetryShort = 61,
    WiphyRetryLong = 62,
    WiphyFragThreshold = 63,
    WiphyRtsThreshold = 64,
    TimedOut = 65,
    UseMfp = 66,
    StaFlags2 = 67,
    ControlPort = 68,
    Testdata = 69,
    Privacy = 70,
    DisconnectedByAp = 71,
    StatusCode = 72,
    CipherSuitesPairwise = 73,
    CipherSuiteGroup = 74,
    WpaVersions = 75,
    AkmSuites = 76,
    ReqIe = 77,
    RespIe = 78,
    PrevBssid = 79,
    Key = 80,
    Keys = 81,
    Pid = 82,
    FourAddr = 83, // NOTE: called NL80211_ATTR_4ADDR
    SurveyInfo = 84,
    Pmkid = 85,
    MaxNumPmkids = 86,
    Duration = 87,
    Cookie = 88,
    WiphyCoverageClass = 89,
    TxRates = 90,
    FrameMatch = 91,
    Ack = 92,
    PsState = 93,
    Cqm = 94,
    LocalStateChange = 95,
    ApIsolate = 96,
    WiphyTxPowerSetting = 97,
    WiphyTxPowerLevel = 98,
    TxFrameTypes = 99,
    RxFrameTypes = 100,
    FrameType = 101,
    ControlPortEthertype = 102,
    ControlPortNoEncrypt = 103,
    SupportIbssRsn = 104,
    WiphyAntennaTx = 105,
    WiphyAntennaRx = 106,
    McastRate = 107,
    OffchannelTxOk = 108,
    BssHtOpmode = 109,
    KeyDefaultTypes = 110,
    MaxRemainOnChannelDuration = 111,
    MeshSetup = 112,
    WiphyAntennaAvailTx = 113,
    WiphyAntennaAvailRx = 114,
    SupportMeshAuth = 115,
    StaPlinkState = 116,
    WowlanTriggers = 117,
    WowlanTriggersSupported = 118,
    SchedScanInterval = 119,
    InterfaceCombinations = 120,
    SoftwareIftypes = 121,
    RekeyData = 122,
    MaxNumSchedScanSsids = 123,
    MaxSchedScanIeLen = 124,
    ScanSuppRates = 125,
    HiddenSsid = 126,
    IeProbeResp = 127,
    IeAssocResp = 128,
    StaWme = 129,
    SupportApUapsd = 130,
    RoamSupport = 131,
    SchedScanMatch = 132,
    MaxMatchSets = 133,
    PmksaCandidate = 134,
    TxNoCckRate = 135,
    TdlsAction = 136,
    TdlsDialogToken = 137,
    TdlsOperation = 138,
    TdlsSupport = 139,
    TdlsExternalSetup = 140,
    DeviceApSme = 141,
    DontWaitForAck = 142,
    FeatureFlags = 143,
    ProbeRespOffload = 144,
    ProbeResp = 145,
    DfsRegion = 146,
    DisableHt = 147,
    HtCapabilityMask = 148,
    NoackMap = 149,
    InactivityTimeout = 150,
    RxSignalDbm = 151,
    BgScanPeriod = 152,
    Wdev = 153,
    UserRegHintType = 154,
    ConnFailedReason = 155,
    AuthData = 156,
    VhtCapability = 157,
    ScanFlags = 158,
    ChannelWidth = 159,
    CenterFreq1 = 160,
    CenterFreq2 = 161,
    P2PCtwindow = 162,
    P2POppps = 163,
    LocalMeshPowerMode = 164,
    AclPolicy = 165,
    MacAddrs = 166,
    MacAclMax = 167,
    RadarEvent = 168,
    ExtCapa = 169,
    ExtCapaMask = 170,
    StaCapability = 171,
    StaExtCapability = 172,
    ProtocolFeatures = 173,
    SplitWiphyDump = 174,
    DisableVht = 175,
    VhtCapabilityMask = 176,
    Mdid = 177,
    IeRic = 178,
    CritProtId = 179,
    MaxCritProtDuration = 180,
    PeerAid = 181,
    CoalesceRule = 182,
    ChSwitchCount = 183,
    ChSwitchBlockTx = 184,
    CsaIes = 185,
    CntdwnOffsBeacon = 186,
    CntdwnOffsPresp = 187,
    RxmgmtFlags = 188,
    StaSupportedChannels = 189,
    StaSupportedOperClasses = 190,
    HandleDfs = 191,
    Support5Mhz = 192,
    Support10Mhz = 193,
    OpmodeNotif = 194,
    VendorId = 195,
    VendorSubcmd = 196,
    VendorData = 197,
    VendorEvents = 198,
    QosMap = 199,
    MacHint = 200,
    WiphyFreqHint = 201,
    MaxApAssocSta = 202,
    TdlsPeerCapability = 203,
    SocketOwner = 204,
    CsaCOffsetsTx = 205,
    MaxCsaCounters = 206,
    TdlsInitiator = 207,
    UseRrm = 208,
    WiphyDynAck = 209,
    Tsid = 210,
    UserPrio = 211,
    AdmittedTime = 212,
    SmpsMode = 213,
    OperClass = 214,
    MacMask = 215,
    WiphySelfManagedReg = 216,
    ExtFeatures = 217,
    SurveyRadioStats = 218,
    NetnsFd = 219,
    SchedScanDelay = 220,
    RegIndoor = 221,
    MaxNumSchedScanPlans = 222,
    MaxScanPlanInterval = 223,
    MaxScanPlanIterations = 224,
    SchedScanPlans = 225,
    Pbss = 226,
    BssSelect = 227,
    StaSupportP2PPs = 228,
    Pad = 229,
    IftypeExtCapa = 230,
    MuMimoGroupData = 231,
    MuMimoFollowMacAddr = 232,
    ScanStartTimeTsf = 233,
    ScanStartTimeTsfBssid = 234,
    MeasurementDuration = 235,
    MeasurementDurationMandatory = 236,
    MeshPeerAid = 237,
    NanMasterPref = 238,
    Bands = 239,
    NanFunc = 240,
    NanMatch = 241,
    FilsKek = 242,
    FilsNonces = 243,
    MulticastToUnicastEnabled = 244,
    Bssid = 245,
    SchedScanRelativeRssi = 246,
    SchedScanRssiAdjust = 247,
    TimeoutReason = 248,
    FilsErpUsername = 249,
    FilsErpRealm = 250,
    FilsErpNextSeqNum = 251,
    FilsErpRrk = 252,
    FilsCacheId = 253,
    Pmk = 254,
    SchedScanMulti = 255,
    SchedScanMaxReqs = 256,
    Want1X4WayHs = 257,
    Pmkr0Name = 258,
    PortAuthorized = 259,
    ExternalAuthAction = 260,
    ExternalAuthSupport = 261,
    Nss = 262,
    AckSignal = 263,
    ControlPortOverNl80211 = 264,
    TxqStats = 265,
    TxqLimit = 266,
    TxqMemoryLimit = 267,
    TxqQuantum = 268,
    HeCapability = 269,
    FtmResponder = 270,
    FtmResponderStats = 271,
    Timeout = 272,
    PeerMeasurements = 273,
    AirtimeWeight = 274,
    StaTxPowerSetting = 275,
    StaTxPower = 276,
    SaePassword = 277,
    TwtResponder = 278,
    HeObssPd = 279,
    WiphyEdmgChannels = 280,
    WiphyEdmgBwConfig = 281,
    VlanId = 282,
    HeBssColor = 283,
    IftypeAkmSuites = 284,
    TidConfig = 285,
    ControlPortNoPreauth = 286,
    PmkLifetime = 287,
    PmkReauthThreshold = 288,
    ReceiveMulticast = 289,
    WiphyFreqOffset = 290,
    CenterFreq1Offset = 291,
    ScanFreqKhz = 292,
    He6GhzCapability = 293,
    FilsDiscovery = 294,
    UnsolBcastProbeResp = 295,
    S1GCapability = 296,
    S1GCapabilityMask = 297,
    SaePwe = 298,
    ReconnectRequested = 299,
    SarSpec = 300,
    DisableHe = 301,
    ObssColorBitmap = 302,
    ColorChangeCount = 303,
    ColorChangeColor = 304,
    ColorChangeElems = 305,
    MbssidConfig = 306,
    MbssidElems = 307,
    RadarBackground = 308,
    ApSettingsFlags = 309,
    EhtCapability = 310,
    DisableEht = 311,
    MloLinks = 312,
    MloLinkId = 313,
    MldAddr = 314,
    MloSupport = 315,
    MaxNumAkmSuites = 316,
    EmlCapability = 317,
    MldCapaAndOps = 318,
    TxHwTimestamp = 319,
    RxHwTimestamp = 320,
    TdBitmap = 321,
    PunctBitmap = 322,
    Max = 322,
}
impl neli::consts::genl::NlAttrType for Nl80211Attribute {}

#[neli::neli_enum(serialized_type = "u16")]
pub enum Nl80211StationInfo {
    Invalid = 0,
    InactiveTime = 1,
    RxBytes = 2,
    TxBytes = 3,
    Llid = 4,
    Plid = 5,
    PlinkState = 6,
    Signal = 7,
    TxBitrate = 8,
    RxPackets = 9,
    TxPackets = 10,
    TxRetries = 11,
    TxFailed = 12,
    SignalAvg = 13,
    RxBitrate = 14,
    BssParam = 15,
    ConnectedTime = 16,
    StaFlags = 17,
    BeaconLoss = 18,
    TOffset = 19,
    LocalPm = 20,
    PeerPm = 21,
    NonpeerPm = 22,
    RxBytes64 = 23,
    TxBytes64 = 24,
    ChainSignal = 25,
    ChainSignalAvg = 26,
    ExpectedThroughput = 27,
    RxDropMisc = 28,
    BeaconRx = 29,
    BeaconSignalAvg = 30,
    TidStats = 31,
    RxDuration = 32,
    Pad = 33,
    AckSignal = 34,
    AckSignalAvg = 35,
    RxMpdus = 36,
    FcsErrorCount = 37,
    ConnectedToGate = 38,
    TxDuration = 39,
    AirtimeWeight = 40,
    AirtimeLinkMetric = 41,
    AssocAtBoottime = 42,
    ConnectedToAs = 43,
}
impl neli::consts::genl::NlAttrType for Nl80211StationInfo {}

#[neli::neli_enum(serialized_type = "u16")]
pub enum Nl80211Bss {
    Invalid = 0,
    Bssid = 1,
    Frequency = 2,
    Tsf = 3,
    BeaconInterval = 4,
    Capability = 5,
    InformationElements = 6,
    SignalMbm = 7,
    SignalUnspec = 8,
    Status = 9,
    SeenMsAgo = 10,
    BeaconIes = 11,
    ChanWidth = 12,
    BeaconTsf = 13,
    PrespData = 14,
    LastSeenBoottime = 15,
    Pad = 16,
    ParentTsf = 17,
    ParentBssid = 18,
    ChainSignal = 19,
    FrequencyOffset = 20,
    MloLinkId = 21,
    MldAddr = 22,
    AfterLast = 23,
    Max = 22,
}
impl neli::consts::genl::NlAttrType for Nl80211Bss {}

#[neli::neli_enum(serialized_type = "u32")]
pub enum Nl80211IfType {
    /// unspecified type, driver decides
    Unspecified = 0,
    /// independent BSS member
    Adhoc = 1,
    /// managed BSS member
    Station = 2,
    /// access point
    Ap = 3,
    /// VLAN interface for access points; VLAN interfaces
    /// are a bit special in that they must always be tied to a pre-existing
    /// AP type interface.
    ApVlan = 4,
    /// wireless distribution interface
    Wds = 5,
    /// monitor interface receiving all frames
    Monitor = 6,
    /// mesh point
    MeshPoint = 7,
    /// P2P client
    P2PClient = 8,
    /// P2P group owner
    P2PGo = 9,
    /// P2P device interface type, this is not a netdev
    /// and therefore can't be created in the normal ways, use the
    /// %NL80211_CMD_START_P2P_DEVICE and %NL80211_CMD_STOP_P2P_DEVICE
    /// commands to create and destroy one
    P2PDevice = 10,
    /// Outside Context of a BSS
    /// This mode corresponds to the MIB variable dot11OCBActivated=true
    Ocb = 11,
    /// NAN device interface type (not a netdev)
    Nan = 12,
}
