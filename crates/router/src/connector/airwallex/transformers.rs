use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use url::Url;
use uuid::Uuid;

use crate::{
    connector::utils,
    core::{errors, payments::operations::Flow},
    pii::{self, Secret},
    services,
    types::{self, api, storage::enums},
};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexIntentRequest {
    // Unique ID to be sent for each transaction/operation request to the connector
    request_id: String,
    amount: String,
    currency: enums::Currency,
    //ID created in merchant's order system that corresponds to this PaymentIntent.
    merchant_order_id: String,
}
impl TryFrom<&types::PaymentsInitRouterData> for AirwallexIntentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsInitRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            amount: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
            currency: item.request.currency,
            merchant_order_id: item.payment_id.clone(),
        })
    }
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexPaymentsRequest {
    // Unique ID to be sent for each transaction/operation request to the connector
    request_id: String,
    payment_method: AirwallexPaymentMethod,
    payment_method_options: Option<AirwallexPaymentOptions>,
    return_url: Option<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum AirwallexPaymentMethod {
    Card(AirwallexCard),
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexCard {
    card: AirwallexCardDetails,
    #[serde(rename = "type")]
    payment_method_type: AirwallexPaymentType,
}
#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexCardDetails {
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    number: Secret<String, pii::CardNumber>,
    cvc: Secret<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AirwallexPaymentType {
    Card,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AirwallexPaymentOptions {
    Card(AirwallexCardPaymentOptions),
}
#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexCardPaymentOptions {
    auto_capture: bool,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for AirwallexPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let mut payment_method_options = None;
        let payment_method = match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ccard) => {
                payment_method_options =
                    Some(AirwallexPaymentOptions::Card(AirwallexCardPaymentOptions {
                        auto_capture: matches!(
                            item.request.capture_method,
                            Some(enums::CaptureMethod::Automatic) | None
                        ),
                    }));
                Ok(AirwallexPaymentMethod::Card(AirwallexCard {
                    card: AirwallexCardDetails {
                        number: ccard
                            .card_number
                            .map(|card| card.split_whitespace().collect()),
                        expiry_month: ccard.card_exp_month.clone(),
                        expiry_year: ccard.card_exp_year.clone(),
                        cvc: ccard.card_cvc,
                    },
                    payment_method_type: AirwallexPaymentType::Card,
                }))
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Unknown payment method".to_string(),
            )),
        }?;
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            payment_method,
            payment_method_options,
            return_url: item.request.complete_authorize_url.clone(),
        })
    }
}

#[derive(Deserialize)]
pub struct AirwallexAuthUpdateResponse {
    #[serde(with = "common_utils::custom_serde::iso8601")]
    expires_at: PrimitiveDateTime,
    token: String,
}

impl<F: Flow, T>
    TryFrom<types::ResponseRouterData<F, AirwallexAuthUpdateResponse, T, types::AccessToken>>
    for types::RouterData<F, T, types::AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, AirwallexAuthUpdateResponse, T, types::AccessToken>,
    ) -> Result<Self, Self::Error> {
        let expires = (item.response.expires_at - common_utils::date_time::now()).whole_seconds();
        Ok(Self {
            response: Ok(types::AccessToken {
                token: item.response.token,
                expires,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexCompleteRequest {
    request_id: String,
    three_ds: AirwallexThreeDsData,
    #[serde(rename = "type")]
    three_ds_type: AirwallexThreeDsType,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexThreeDsData {
    acs_response: Option<common_utils::pii::SecretSerdeValue>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub enum AirwallexThreeDsType {
    #[default]
    #[serde(rename = "3ds_continue")]
    ThreeDSContinue,
}

impl TryFrom<&types::PaymentsCompleteAuthorizeRouterData> for AirwallexCompleteRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCompleteAuthorizeRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            three_ds: AirwallexThreeDsData {
                acs_response: item.request.payload.clone().map(Secret::new),
            },
            three_ds_type: AirwallexThreeDsType::ThreeDSContinue,
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexPaymentsCaptureRequest {
    // Unique ID to be sent for each transaction/operation request to the connector
    request_id: String,
    amount: Option<String>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for AirwallexPaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            amount: Some(utils::to_currency_base_unit(
                item.request.amount_to_capture,
                item.request.currency,
            )?),
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexPaymentsCancelRequest {
    // Unique ID to be sent for each transaction/operation request to the connector
    request_id: String,
    cancellation_reason: Option<String>,
}

impl TryFrom<&types::PaymentsCancelRouterData> for AirwallexPaymentsCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            cancellation_reason: item.request.cancellation_reason.clone(),
        })
    }
}

// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AirwallexPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Pending,
    RequiresPaymentMethod,
    RequiresCustomerAction,
    RequiresCapture,
    Cancelled,
}

fn get_payment_status(response: &AirwallexPaymentsResponse) -> enums::AttemptStatus {
    match response.status.clone() {
        AirwallexPaymentStatus::Succeeded => enums::AttemptStatus::Charged,
        AirwallexPaymentStatus::Failed => enums::AttemptStatus::Failure,
        AirwallexPaymentStatus::Pending => enums::AttemptStatus::Pending,
        AirwallexPaymentStatus::RequiresPaymentMethod => enums::AttemptStatus::PaymentMethodAwaited,
        AirwallexPaymentStatus::RequiresCustomerAction => response.next_action.as_ref().map_or(
            enums::AttemptStatus::AuthenticationPending,
            |next_action| match next_action.stage {
                AirwallexNextActionStage::WaitingDeviceDataCollection => {
                    enums::AttemptStatus::DeviceDataCollectionPending
                }
                AirwallexNextActionStage::WaitingUserInfoInput => {
                    enums::AttemptStatus::AuthenticationPending
                }
            },
        ),
        AirwallexPaymentStatus::RequiresCapture => enums::AttemptStatus::Authorized,
        AirwallexPaymentStatus::Cancelled => enums::AttemptStatus::Voided,
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AirwallexNextActionStage {
    WaitingDeviceDataCollection,
    WaitingUserInfoInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AirwallexRedirectFormData {
    #[serde(rename = "JWT")]
    jwt: Option<String>,
    #[serde(rename = "threeDSMethodData")]
    three_ds_method_data: Option<String>,
    token: Option<String>,
    provider: Option<String>,
    version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AirwallexPaymentsNextAction {
    url: Url,
    method: services::Method,
    data: AirwallexRedirectFormData,
    stage: AirwallexNextActionStage,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AirwallexPaymentsResponse {
    status: AirwallexPaymentStatus,
    //Unique identifier for the PaymentIntent
    id: String,
    amount: Option<f32>,
    //ID of the PaymentConsent related to this PaymentIntent
    payment_consent_id: Option<String>,
    next_action: Option<AirwallexPaymentsNextAction>,
}

fn get_redirection_form(
    response_url_data: AirwallexPaymentsNextAction,
) -> Option<services::RedirectForm> {
    Some(services::RedirectForm {
        endpoint: response_url_data.url.to_string(),
        method: response_url_data.method,
        form_fields: std::collections::HashMap::from([
            //Some form fields might be empty based on the authentication type by the connector
            (
                "JWT".to_string(),
                response_url_data.data.jwt.unwrap_or_default(),
            ),
            (
                "threeDSMethodData".to_string(),
                response_url_data
                    .data
                    .three_ds_method_data
                    .unwrap_or_default(),
            ),
            (
                "token".to_string(),
                response_url_data.data.token.unwrap_or_default(),
            ),
            (
                "provider".to_string(),
                response_url_data.data.provider.unwrap_or_default(),
            ),
            (
                "version".to_string(),
                response_url_data.data.version.unwrap_or_default(),
            ),
        ]),
    })
}

impl<F: Flow, T>
    TryFrom<types::ResponseRouterData<F, AirwallexPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            AirwallexPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (status, redirection_data) = item.response.next_action.clone().map_or(
            // If no next action is there, map the status and set redirection form as None
            (get_payment_status(&item.response), None),
            |response_url_data| {
                // If the connector sends a customer action response that is already under
                // process from our end it can cause an infinite loop to break this this check
                // is added and fail the payment
                if matches!(
                    (
                        response_url_data.stage.clone(),
                        item.data.status,
                        item.response.status.clone(),
                    ),
                    // If the connector sends waiting for DDC and our status is already DDC Pending
                    // that means we initiated the call to collect the data and now we expect a different response
                    (
                        AirwallexNextActionStage::WaitingDeviceDataCollection,
                        enums::AttemptStatus::DeviceDataCollectionPending,
                        _
                    )
                    // If the connector sends waiting for Customer Action and our status is already Authenticaition Pending
                    // that means we initiated the call to authenticate and now we do not expect a requires_customer action
                    // it will start a loop
                    | (
                        _,
                        enums::AttemptStatus::AuthenticationPending,
                        AirwallexPaymentStatus::RequiresCustomerAction,
                    )
                ) {
                    // Fail the payment for above conditions
                    (enums::AttemptStatus::AuthenticationFailed, None)
                } else {
                    (
                        //Build the redirect form and update the payment status
                        get_payment_status(&item.response),
                        get_redirection_form(response_url_data),
                    )
                }
            },
        );
        Ok(Self {
            status,
            reference_id: Some(item.response.id.clone()),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct AirwallexRefundRequest {
    // Unique ID to be sent for each transaction/operation request to the connector
    request_id: String,
    amount: Option<String>,
    reason: Option<String>,
    //Identifier for the PaymentIntent for which Refund is requested
    payment_intent_id: String,
}

impl<F: Flow> TryFrom<&types::RefundsRouterData<F>> for AirwallexRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            amount: Some(utils::to_currency_base_unit(
                item.request.refund_amount,
                item.request.currency,
            )?),
            reason: item.request.reason.clone(),
            payment_intent_id: item.request.connector_transaction_id.clone(),
        })
    }
}

// Type definition for Refund Response
#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Received,
    Accepted,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Received | RefundStatus::Accepted => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    //A unique number that tags a credit or debit card transaction when it goes from the merchant's bank through to the cardholder's bank.
    acquirer_reference_number: String,
    amount: f32,
    //Unique identifier for the Refund
    id: String,
    status: RefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AirwallexWebhookData {
    pub source_id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AirwallexWebhookDataResource {
    pub object: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct AirwallexWebhookObjectResource {
    pub data: AirwallexWebhookDataResource,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct AirwallexErrorResponse {
    pub code: String,
    pub message: String,
    pub source: Option<String>,
}
